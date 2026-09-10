// src/analytics/doing/meeting.rs
//! What passes between two people.
//!
//! Talking, telling, mating, asking, trading, taking, giving, and going
//! without so that somebody else does not have to.
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

    /// Reachable from the tests under the same name.
    #[cfg(test)]
    pub(in crate::analytics) fn hand_over_for_test(
        &mut self,
        from: usize,
        to: usize,
        item_id: &str,
        how_many: u32,
    ) -> u32 {
        self.hand_over(from, to, item_id, how_many)
    }

    /// Move a stack out of one pack and into another, whole.
    ///
    /// Every one of the four places that handed something over built the
    /// receiving stack from scratch - `new_with_weight(name, how_many, 1.0)` -
    /// so what arrived had the right name and nothing else. Food lost its
    /// nutrition, its freshness and its preparation state; a dried strip
    /// became an anonymous handful, and because an untracked stack has no
    /// freshness it then never spoiled either. It also weighed a flat one
    /// whatever it was, against food's real half, so a traded meal weighed
    /// double against a pack that holds twelve.
    ///
    /// A gift is the same thing in somebody else's hands. Take it across
    /// whole: same weight, same food data, same quality and durability.
    ///
    /// Returns how many actually went across, which is nought if the
    /// receiver's pack would not take them - what will not go in stays with
    /// the giver rather than vanishing.
    pub(in crate::analytics) fn hand_over(
        &mut self,
        from: usize,
        to: usize,
        item_id: &str,
        how_many: u32,
    ) -> u32 {
        let Some(stack) = self.population.agents[from]
            .inventory
            .get_item(item_id)
            .cloned()
        else {
            return 0;
        };

        let going = how_many.min(stack.quantity);
        if going == 0 {
            return 0;
        }

        let mut handed = stack.clone();
        handed.quantity = going;

        if !self.population.agents[to].inventory.add_item(handed) {
            return 0;
        }

        self.population.agents[from]
            .inventory
            .remove_item(item_id, going);
        going
    }
    /// `Action::Socialize`.
    pub(in crate::analytics) fn socialising(&mut self, target_agent_id: &uuid::Uuid, agent_index: usize, rng: &mut rand::rngs::StdRng, tick_now: u32) -> ActionResult {
        use crate::agents::social_interactions::{
            SocialInteractionType, HelpType,
            calculate_relationship_change, calculate_social_satisfaction,
            should_greet, select_conversation_topic, calculate_gift_value, would_accept_gift
        };
        use crate::core::traits::Trait;

        // Find the target agent
        let target_index = self.population.agents.iter().position(|a| a.id == *target_agent_id);
        if target_index.is_none() {
            return ActionResult::failure("Target agent not found".to_string());
        }
        let target_index = target_index.unwrap();

        // Don't socialize with self
        if target_index == agent_index {
            return ActionResult::failure("Cannot socialize with self".to_string());
        }

        // Near enough to say it to. The target is resolved before this, and
        // could be anybody in the settlement; whether they are actually within
        // earshot is this verb's own business - see
        // `Simulation::WITHIN_TALKING_DISTANCE`.
        if !Self::near_enough_to_talk(
            self.population.agents[agent_index].state.position,
            self.population.agents[target_index].state.position,
        ) {
            return ActionResult::failure("Too far off to say anything".to_string());
        }


        // Get relationship data (clone to avoid borrow issues)
        let initiator_traits: Vec<Trait> = self.population.agents[agent_index]
            .traits.get_traits().iter().copied().collect();
        let recipient_traits: Vec<Trait> = self.population.agents[target_index]
            .traits.get_traits().iter().copied().collect();

        // Get or create relationship
        let current_tick = self.current_tick;
        let initiator_agent = &mut self.population.agents[agent_index];
        let relationship = initiator_agent.relationships
            .get_or_create_relationship(*target_agent_id, current_tick);

        let current_relationship = relationship.relationship_level();
        let current_trust = relationship.trust_level();
        let last_interaction_tick = relationship.last_interaction_tick;

        // Determine interaction type based on relationship and context
        let interaction_type = if should_greet(last_interaction_tick, current_tick, &current_relationship) {
            // Greet if haven't interacted in a while
            SocialInteractionType::Greet
        } else {
            // Choose conversation or other interaction based on relationship
            let choice = rng.gen_range(0..100);

            match &current_relationship {
                crate::agents::relationships::RelationshipLevel::Loves(_) => {
                    // Close relationships: more variety
                    if choice < 40 {
                        let topic = select_conversation_topic(&current_relationship, &initiator_traits, &recipient_traits);
                        SocialInteractionType::Converse { topic }
                    } else if choice < 60 {
                        SocialInteractionType::ShareMeal
                    } else if choice < 75 {
                        SocialInteractionType::Compliment
                    } else if choice < 90 {
                        SocialInteractionType::OfferHelp {
                            help_type: HelpType::General,
                        }
                    } else {
                        // Try to give a gift if we have something
                        let initiator = &self.population.agents[agent_index];
                        if let Some((item_id, item)) = initiator.inventory.get_all_items().iter().next() {
                            if item.quantity > 1 {
                                // Map item_id string to ItemType
                                let item_type = match item_id.to_lowercase().as_str() {
                                    "wood" => crate::world::ItemType::Wood,
                                    "stone" => crate::world::ItemType::Stone,
                                    "iron" => crate::world::ItemType::Iron,
                                    "food" => crate::world::ItemType::Food,
                                    "bread" => crate::world::ItemType::Bread,
                                    _ => crate::world::ItemType::Wood, // Default
                                };
                                SocialInteractionType::GiveGift {
                                    item_type,
                                    quantity: 1,
                                }
                            } else {
                                let topic = select_conversation_topic(&current_relationship, &initiator_traits, &recipient_traits);
                                SocialInteractionType::Converse { topic }
                            }
                        } else {
                            let topic = select_conversation_topic(&current_relationship, &initiator_traits, &recipient_traits);
                            SocialInteractionType::Converse { topic }
                        }
                    }
                }
                crate::agents::relationships::RelationshipLevel::Likes(_) => {
                    // Friends: mostly conversation and help
                    if choice < 60 {
                        let topic = select_conversation_topic(&current_relationship, &initiator_traits, &recipient_traits);
                        SocialInteractionType::Converse { topic }
                    } else if choice < 80 {
                        SocialInteractionType::Compliment
                    } else {
                        SocialInteractionType::OfferHelp {
                            help_type: HelpType::General,
                        }
                    }
                }
                _ => {
                    // Neutral or negative: stick to safe interactions
                    if choice < 80 {
                        let topic = select_conversation_topic(&current_relationship, &initiator_traits, &recipient_traits);
                        SocialInteractionType::Converse { topic }
                    } else {
                        SocialInteractionType::ThankYou
                    }
                }
            }
        };

        // Calculate interaction effects
        let relationship_change = calculate_relationship_change(
            &interaction_type,
            &initiator_traits,
            &recipient_traits,
            &current_relationship,
        );

        let social_satisfaction = calculate_social_satisfaction(
            &interaction_type,
            &initiator_traits,
            &current_relationship,
        );

        // Handle gift giving specially (may fail if rejected)
        let mut success = true;
        let message;

        match &interaction_type {
            SocialInteractionType::GiveGift { item_type, quantity } => {
                // Check if gift would be accepted
                if would_accept_gift(&current_relationship, &current_trust, &recipient_traits) {
                    // Format item_type as string for inventory operations
                    let item_str = format!("{:?}", item_type).to_lowercase();

                    // Remove from initiator inventory
                    let initiator = &mut self.population.agents[agent_index];
                    if let Some(given) = initiator.inventory.remove_item(&item_str, *quantity) {
                        // What is handed over is the thing itself.
                        //
                        // It used to be a *new* item built out of the
                        // name: same id, same count, and nothing else
                        // - no food data, no freshness, no
                        // preparation, and a flat weight of 2.0
                        // whatever it was. So giving somebody a week-old
                        // fish handed them a fish that would never go
                        // off, and giving away a dried strip threw the
                        // drying away. See ISSUES_FOUND #61.
                        let recipient = &mut self.population.agents[target_index];
                        recipient.inventory.add_item(given);

                        let gift_value = calculate_gift_value(item_type, *quantity);
                        message = format!("Gave {} {:?} to agent (value: {:.1})", quantity, item_type, gift_value);
                        success = true;
                    } else {
                        message = "Don't have enough to give as gift".to_string();
                        success = false;
                    }
                } else {
                    message = "Gift was politely refused".to_string();
                    success = false;
                }
            }
            SocialInteractionType::Greet => {
                message = format!("Greeted agent (relationship: {:?})", current_relationship);
            }
            SocialInteractionType::Converse { topic } => {
                message = format!("Had conversation about {:?}", topic);
            }
            SocialInteractionType::OfferHelp { help_type } => {
                message = format!("Offered {:?} help", help_type);
            }
            SocialInteractionType::ThankYou => {
                message = "Expressed gratitude".to_string();
            }
            SocialInteractionType::Compliment => {
                message = "Gave a compliment".to_string();
            }
            SocialInteractionType::ShareMeal => {
                message = "Shared a meal together".to_string();
            }
        }

        // Update initiator's relationship
        let initiator = &mut self.population.agents[agent_index];
        let relationship = initiator.relationships
            .get_or_create_relationship(*target_agent_id, current_tick);

        if success && relationship_change != 0 {
            if relationship_change > 0 {
                relationship.positive_interaction(relationship_change, current_tick);
            } else {
                relationship.negative_interaction(relationship_change.abs(), current_tick);
            }
        }
        relationship.last_interaction_tick = current_tick;
        relationship.total_interactions += 1;

        // Also update target's relationship (reciprocal, but may differ based on their traits)
        let target_relationship_change = calculate_relationship_change(
            &interaction_type,
            &recipient_traits,
            &initiator_traits,
            &current_relationship,
        );

        // Capture initiator's ID before mutable borrow
        let initiator_id = self.population.agents[agent_index].id;

        let target = &mut self.population.agents[target_index];
        let target_relationship = target.relationships
            .get_or_create_relationship(initiator_id, current_tick);

        if success && target_relationship_change != 0 {
            if target_relationship_change > 0 {
                target_relationship.positive_interaction(target_relationship_change, current_tick);
            } else {
                target_relationship.negative_interaction(target_relationship_change.abs(), current_tick);
            }
        }
        target_relationship.last_interaction_tick = current_tick;
        target_relationship.total_interactions += 1;

        // Calculate target's social satisfaction too
        let target_satisfaction = calculate_social_satisfaction(
            &interaction_type,
            &recipient_traits,
            &current_relationship,
        );

        // Update target's social drive
        let target = &mut self.population.agents[target_index];
        if let Some(social_drive) = target.drives.get_mut(DriveType::Social) {
            social_drive.decrease(target_satisfaction);
        }

        if success {
            // Grant Social skill XP
            let initiator = &mut self.population.agents[agent_index];
            initiator.skills.practise(crate::agents::skills::SkillType::Social, 1, tick_now);

            // Record that this agent satisfied our social drive
            let tick = self.current_tick;
            let initiator = &mut self.population.agents[agent_index];
            initiator.record_drive_satisfaction(DriveType::Social, *target_agent_id, social_satisfaction, tick);

            // Helper happiness for initiator (providing social satisfaction to target)
            let initiator = &mut self.population.agents[agent_index];
            initiator.process_helper_happiness(*target_agent_id, target_satisfaction);

            // Also record for the target (reciprocal satisfaction)
            let target = &mut self.population.agents[target_index];
            target.record_drive_satisfaction(DriveType::Social, initiator_id, target_satisfaction, tick);

            // Helper happiness for target (providing social satisfaction to initiator)
            let target = &mut self.population.agents[target_index];
            target.process_helper_happiness(initiator_id, social_satisfaction);

            debug!(
                "Agent {} socialized with agent {}: {} (relationship change: {:+}, satisfaction: {:.2})",
                initiator_id,
                target_agent_id,
                message,
                relationship_change,
                social_satisfaction
            );

            ActionResult::success()
                .with_drive_change(DriveType::Social, -social_satisfaction)
                .with_energy_cost(3.0)
                .with_message(message)
        } else {
            ActionResult::failure(message)
        }
    }

    /// `Action::ShareInformation`.
    pub(in crate::analytics) fn sharing_information(&mut self, target_agent_id: &uuid::Uuid, agent_index: usize, rng: &mut rand::rngs::StdRng) -> ActionResult {
        use crate::agents::gossip::{Information, InformationType};
        use crate::core::traits::Trait;

        // Find the target agent
        let target_index = self.population.agents.iter().position(|a| a.id == *target_agent_id);
        if target_index.is_none() {
            return ActionResult::failure("Target agent not found".to_string());
        }
        let target_index = target_index.unwrap();

        // Don't share with self
        if target_index == agent_index {
            return ActionResult::failure("Cannot share information with self".to_string());
        }

        // Near enough to say it to - see `Simulation::WITHIN_TALKING_DISTANCE`.
        if !Self::near_enough_to_talk(
            self.population.agents[agent_index].state.position,
            self.population.agents[target_index].state.position,
        ) {
            return ActionResult::failure("Too far off to say anything".to_string());
        }

        let current_tick = self.current_tick;

        // Capture initiator data before mutable borrows
        let (initiator_id, info_to_share) = {
            let initiator = &self.population.agents[agent_index];
            let _initiator_traits: Vec<Trait> = initiator.traits.get_traits().iter().copied().collect();
            let initiator_id = initiator.id;

            // Select information to share from initiator's knowledge base
            let info = if !initiator.knowledge.known_information.is_empty() {
                // Pick a random piece of information from their knowledge
                let info_list: Vec<_> = initiator.knowledge.known_information.values().collect();
                let idx = rng.gen_range(0..info_list.len());
                let original_info = info_list[idx].clone();

                // Check if initiator would distort information based on traits
                if let Some(distortion_trait) = initiator.traits.would_distort_info() {
                    // Distort the information
                    original_info.distort(distortion_trait, initiator_id)
                } else {
                    // Share truthfully
                    original_info
                }
            } else {
                // Generate new information if they don't have any
                // Share a resource location they might know about
                let agent_pos = initiator.state.position;
                Information::new(
                    InformationType::ResourceLocation {
                        resource: "generic".to_string(),
                        location: agent_pos,
                    },
                    initiator_id,
                    true, // Assume they know their current location
                    current_tick as u64,
                )
            };

            (initiator_id, info)
        };

        // Get recipient's traits for belief calculation
        let recipient_traits = self.population.agents[target_index].traits.clone();

        // Share the information with recipient
        let target = &mut self.population.agents[target_index];
        let target_id = target.id;
        target.knowledge.receive_information(
            info_to_share.clone(),
            initiator_id,
            target_id,
            &recipient_traits,
            current_tick as u64,
        );

        // Determine message based on information type
        let message = match &info_to_share.info_type {
            InformationType::ResourceLocation { resource, location } => {
                format!("Shared knowledge about {} at ({}, {}, {})",
                    resource, location.0, location.1, location.2)
            }
            InformationType::Conflict { agent1: _, agent2: _ } => {
                format!("Gossiped about conflict between agents")
            }
            InformationType::Death { agent: _, cause } => {
                format!("Shared news of death: {}", cause)
            }
            InformationType::TechnologyDiscovered { tech } => {
                format!("Shared discovery of {} technology", tech)
            }
            InformationType::Accusation {  crime, .. } => {
                format!("Shared accusation of {}", crime)
            }
            _ => "Shared information".to_string(),
        };

        // Distortion affects satisfaction
        let distortion_penalty = if info_to_share.distortion.is_some() { 0.05 } else { 0.0 };
        let social_satisfaction = 0.15 - distortion_penalty;

        debug!(
            "Agent {} shared information with agent {} (distorted: {}, reliability: {:.2})",
            initiator_id,
            target_agent_id,
            info_to_share.distortion.is_some(),
            info_to_share.reliability
        );

        ActionResult::success()
            .with_drive_change(DriveType::Social, -social_satisfaction)
            .with_energy_cost(2.0)
            .with_message(message)
    }

    /// `Action::Mate`.
    pub(in crate::analytics) fn mating(&mut self, target_agent_id: &uuid::Uuid, agent_index: usize, rng: &mut rand::rngs::StdRng) -> ActionResult {
        use crate::agents::reproduction::{can_mate, MateSelectionCriteria};
        use crate::agents::gossip::{Information, InformationType};

        // Find the target agent
        let target_index = self.population.agents.iter().position(|a| a.id == *target_agent_id);
        if target_index.is_none() {
            return ActionResult::failure("Target agent not found".to_string());
        }
        let target_index = target_index.unwrap();

        // Don't mate with self
        if target_index == agent_index {
            return ActionResult::failure("Cannot mate with self".to_string());
        }

        // Check if both agents can mate
        let initiator = &self.population.agents[agent_index];
        let target = &self.population.agents[target_index];
        let criteria = MateSelectionCriteria::default();

        if !can_mate(initiator, target, &criteria) {
            // Determine specific reason for failure
            let reason = if !initiator.can_reproduce() {
                "Initiator cannot reproduce (too young, too old, or pregnant)".to_string()
            } else if !target.can_reproduce() {
                "Target cannot reproduce (too young, too old, or pregnant)".to_string()
            } else if initiator.fertility() < criteria.min_fertility {
                format!("Initiator fertility too low ({:.2})", initiator.fertility())
            } else if target.fertility() < criteria.min_fertility {
                format!("Target fertility too low ({:.2})", target.fertility())
            } else if target.parent_ids.contains(&initiator.id) || initiator.parent_ids.contains(&target.id) {
                "Cannot mate with parent/child".to_string()
            } else {
                "Agents too far apart for mating".to_string()
            };

            return ActionResult::failure(reason);
        }

        // Calculate mating success probability based on relationship
        let initiator_id = initiator.id;
        let target_id = target.id;
        let mut success_probability = 0.5; // Base 50% chance

        // Check relationship - better relationships increase success
        if let Some(relationship) = initiator.relationships.get_relationship(&target_id) {
            match &relationship.relationship_level() {
                crate::agents::relationships::RelationshipLevel::Loves(_) => {
                    success_probability = 0.9; // High success with loved ones
                }
                crate::agents::relationships::RelationshipLevel::Likes(_) => {
                    success_probability = 0.7; // Good success with friends
                }
                crate::agents::relationships::RelationshipLevel::Neutral(_) => {
                    success_probability = 0.5; // Neutral success
                }
                _ => {
                    success_probability = 0.2; // Low success with dislikes/hates
                }
            }
        }

        // Attempt mating
        if rng.gen_bool(success_probability as f64) {
            // Mating successful - decide who carries it and attempt impregnation
            use crate::agents::reproduction::attempt_impregnation;

            let initiator = &self.population.agents[agent_index];
            let target = &self.population.agents[target_index];

            // Which of the two carries it. There is no gender in this model,
            // so this is not a property of either of them: the lower id, the
            // same rule the population's own pairing pass uses, so that a pair
            // gets the same answer whichever of them started it.
            let (female_index, male_index) = if initiator.id <= target.id {
                (agent_index, target_index)
            } else {
                (target_index, agent_index)
            };

            // Attempt impregnation
            let male = &self.population.agents[male_index];
            let female = &self.population.agents[female_index];
            let current_tick = self.current_tick;

            if let Some(pregnancy) = attempt_impregnation(female, male, current_tick) {
                // Pregnancy started!
                let female = &mut self.population.agents[female_index];
                female.pregnancy = Some(pregnancy);

                debug!(
                    "Agent {} and agent {} mated - the second is carrying it",
                    self.population.agents[male_index].id,
                    self.population.agents[female_index].id
                );

                // Generate gossip about the pregnancy
                let female_id = self.population.agents[female_index].id;
                let female_pos = self.population.agents[female_index].state.position;
                let pregnancy_info = Information::new(
                    InformationType::Pregnancy {
                        agent: female_id,
                    },
                    female_id,
                    true,
                    current_tick as u64,
                );

                // Share pregnancy information with nearby agents
                for other_agent in &mut self.population.agents {
                    if other_agent.id != initiator_id && other_agent.id != target_id {
                        let distance = {
                            let dx = (other_agent.state.position.0 - female_pos.0) as f32;
                            let dy = (other_agent.state.position.1 - female_pos.1) as f32;
                            (dx * dx + dy * dy).sqrt()
                        };

                        if distance <= 15.0 {
                            other_agent.knowledge.receive_information(
                                pregnancy_info.clone(),
                                female_id,
                                other_agent.id,
                                &other_agent.traits,
                                current_tick as u64,
                            );
                        }
                    }
                }

                // Update reproduction drives for both parents
                let male = &mut self.population.agents[male_index];
                if let Some(repro_drive) = male.drives.get_mut(DriveType::Reproduction) {
                    repro_drive.decrease(0.5); // Male drive reduces moderately
                }

                let female = &mut self.population.agents[female_index];
                if let Some(repro_drive) = female.drives.get_mut(DriveType::Reproduction) {
                    repro_drive.decrease(0.9); // Female drive significantly reduces (pregnant)
                }

                ActionResult::success()
                    .with_drive_change(DriveType::Reproduction, -0.7)
                    .with_energy_cost(15.0)
                    .with_message("Mating successful - pregnancy started!".to_string())
            } else {
                // Conception failed (fertility roll failed)
                debug!(
                    "Agent {} and agent {} mated but conception failed",
                    initiator_id, target_id
                );

                // Still reduce drives somewhat
                let agent = &mut self.population.agents[agent_index];
                if let Some(repro_drive) = agent.drives.get_mut(DriveType::Reproduction) {
                    repro_drive.decrease(0.3);
                }

                let target = &mut self.population.agents[target_index];
                if let Some(repro_drive) = target.drives.get_mut(DriveType::Reproduction) {
                    repro_drive.decrease(0.3);
                }

                ActionResult::success()
                    .with_drive_change(DriveType::Reproduction, -0.3)
                    .with_energy_cost(10.0)
                    .with_message("Mating occurred but no conception".to_string())
            }
        } else {
            // Mating attempt rejected
            debug!(
                "Agent {} mating attempt with agent {} was rejected",
                initiator_id, target_id
            );

            ActionResult::failure("Mating attempt rejected by partner".to_string())
        }
    }

    /// `Action::TakeFrom`.
    pub(in crate::analytics) fn taking_from(&mut self, from: &uuid::Uuid, agent_index: usize, tick_now: u32) -> ActionResult {
        let Some(them) = self
            .population
            .agents
            .iter()
            .position(|other| other.id == *from && other.state.is_alive)
        else {
            return ActionResult::failure("Nobody there to take from".to_string());
        };

        // What they have that I am short of. The same question a trade
        // asks, with the asking left out.
        let Some(theirs) = self.what_i_would_hand_over(them, agent_index) else {
            return ActionResult::failure("Nothing of theirs worth taking".to_string());
        };

        let took = (theirs.1 / 2).max(1);
        let me = self.population.agents[agent_index].id;
        let robbed = self.population.agents[them].id;

        // What is taken is the thing itself, clock and all. Building
        // a fresh item out of the name handed the thief a stack that
        // would never go off - stealing a week-old fish got you a
        // fish that keeps for ever. See ISSUES_FOUND #61.
        let how_strong_the_thief_is = self.population.agents[agent_index].own_strength();

        let taken = {
            let other = &mut self.population.agents[them];
            let taken = other.inventory.remove_item(&theirs.0, took);
            other.they_took_something_of_mine(me, &theirs.0, took, tick_now, how_strong_the_thief_is);
            taken
        };

        if let Some(taken) = taken {
            let agent = &mut self.population.agents[agent_index];
            agent.inventory.add_item(taken);
        }

        // And whoever else was standing there saw it. A thief in a
        // camp of forty is a thief to forty people.
        let here = self.population.agents[agent_index].state.position;
        let mut who_saw_it = 0;

        for onlooker in 0..self.population.agents.len() {
            if onlooker == agent_index || onlooker == them {
                continue;
            }
            if !self.population.agents[onlooker].state.is_alive {
                continue;
            }

            let stood = self.population.agents[onlooker].state.position;
            let apart = (stood.0 - here.0).abs().max((stood.1 - here.1).abs());

            if apart <= Self::CLOSE_ENOUGH_TO_SEE_IT_COME_UP {
                who_saw_it += 1;
                self.population.agents[onlooker]
                    .they_took_something_of_mine(
                        me,
                        &theirs.0,
                        took,
                        tick_now,
                        how_strong_the_thief_is,
                    );

                // And the watcher learns something about taking, without
                // having had to try it. This is the second of the three
                // places worry comes from - the agent's own history, what it
                // saw happen to somebody else, and what it took from whoever
                // raised it - and it is what stops every generation having to
                // be caught once before it knows there is anything to be
                // caught at.
                self.population.agents[onlooker].patterns.taught_to_dread(
                    DriveType::Utility,
                    crate::agents::patterns::Element::Did("takefrom".to_string()),
                    DriveType::Social,
                    crate::agents::patterns::Patterns::WHAT_WATCHING_IT_HAPPEN_TEACHES,
                );
            }
        }

        // And what it cost the thief, which is the whole of why anybody would
        // hesitate. Standing is not lost by taking; it is lost by being seen
        // to take. A man who steals in an empty camp learns that stealing is
        // free, his worry fades on the ordinary clock, and the next time he is
        // hungry he steals sooner - which is the behaviour wanted, not a
        // defect in it. See `Patterns::it_cost_me`.
        if who_saw_it > 0 {
            let cost = (who_saw_it as f32 * Self::WHAT_ONE_PAIR_OF_EYES_COSTS)
                .min(crate::agents::patterns::Patterns::WHAT_ONE_CONSEQUENCE_IS_WORTH);
            self.population.agents[agent_index].this_cost_me(DriveType::Social, cost, tick_now);
        }

        debug!("Agent {me} helped himself to {took} {} of {robbed}'s", theirs.0);

        ActionResult::success()
            .with_drive_change(DriveType::Utility, -0.35)
            .with_energy_cost(2.0)
            .with_message(format!("Took {took} {} from somebody", theirs.0))
    }

    /// `Action::AskAbout`.
    pub(in crate::analytics) fn asking_about(&mut self, who: &uuid::Uuid, what: &String, agent_index: usize) -> ActionResult {
        let Some(them) = self
            .population
            .agents
            .iter()
            .position(|other| other.id == *who && other.state.is_alive)
        else {
            return ActionResult::failure("Nobody there to ask".to_string());
        };

        if them == agent_index {
            return ActionResult::failure("Asking yourself teaches nothing".to_string());
        }

        let Some(teaches) = self.what_asking_about_would_teach(them, what) else {
            return ActionResult::failure(format!(
                "They could not say how the {what} came about"
            ));
        };

        // Whether this one takes their word for it. A settlement where
        // everybody believes everybody is a settlement one liar can
        // ruin, and the machinery for deciding whose word is worth
        // anything has been there since the gossip work.
        let their_traits = self.population.agents[them].traits.clone();
        let believed = {
            let asker = &self.population.agents[agent_index];
            asker.would_take_their_word(*who, &their_traits)
        };

        if !believed {
            return ActionResult::failure(format!(
                "Would not take their word about the {what}"
            ));
        }

        let told_me_something_new =
            self.population.agents[agent_index].found_out_how_to(&teaches);

        if told_me_something_new {
            *self.what_anybody_was_told.entry(teaches.clone()).or_insert(0) += 1;
            debug!(
                "Agent {} was told about {what} by {who}",
                self.population.agents[agent_index].id
            );
        }

        // Both of them got something out of it. Being asked after a
        // thing you worked out is the one moment in this model where
        // having worked something out is worth anything socially.
        ActionResult::success()
            .with_drive_change(DriveType::Curiosity, -0.3)
            .with_drive_change(DriveType::Social, -0.1)
            .with_energy_cost(1.0)
            .with_message(format!("Asked about the {what}"))
    }

    /// `Action::Trade`.
    pub(in crate::analytics) fn trading(&mut self, with: &uuid::Uuid, agent_index: usize, tick_now: u32) -> ActionResult {
        let Some(them) = self
            .population
            .agents
            .iter()
            .position(|other| other.id == *with && other.state.is_alive)
        else {
            return ActionResult::failure("Nobody there to trade with".to_string());
        };

        let Some((mine, theirs)) = self.what_the_two_of_them_would_swap(agent_index, them)
        else {
            return ActionResult::failure(
                "Nothing between us that either of us wants".to_string(),
            );
        };

        // Half of what each has spare, so a trade leaves both better
        // off and neither stripped
        let how_much = |spare: u32| (spare / 2).max(1);
        let i_hand_over = how_much(mine.1);
        let they_hand_over = how_much(theirs.1);

        // Both stacks go across whole. A trade that came to nothing on either
        // side is not a trade: if one pack will not take what is offered, the
        // other keeps what it had.
        let they_gave = self.hand_over(them, agent_index, &theirs.0, they_hand_over);
        if they_gave == 0 {
            return ActionResult::failure(
                "No room in the pack for what they offered".to_string(),
            );
        }
        let i_gave = self.hand_over(agent_index, them, &mine.0, i_hand_over);
        if i_gave == 0 {
            // Put theirs back, so nobody is left holding a one-sided bargain.
            self.hand_over(agent_index, them, &theirs.0, they_gave);
            return ActionResult::failure(
                "No room in their pack for what was offered".to_string(),
            );
        }

        {
            let agent = &mut self.population.agents[agent_index];
            agent.skills.practise(crate::agents::SkillType::Social, 8, tick_now);
        }

        let me = self.population.agents[agent_index].id;
        let them_id = self.population.agents[them].id;

        {
            let other = &mut self.population.agents[them];
            other.skills.practise(crate::agents::SkillType::Social, 8, tick_now);

            // A good trade is a good turn on both sides, and both
            // remember who it was with
            other.they_did_me_a_good_turn(me, Self::WHAT_A_FAIR_TRADE_IS_WORTH);
        }

        self.population.agents[agent_index]
            .they_did_me_a_good_turn(them_id, Self::WHAT_A_FAIR_TRADE_IS_WORTH);

        debug!(
            "Agent {me} gave {them_id} {i_hand_over} {} for {they_hand_over} {}",
            mine.0, theirs.0
        );

        ActionResult::success()
            .with_drive_change(DriveType::Utility, -0.35)
            .with_drive_change(DriveType::Social, -0.15)
            .with_energy_cost(2.0)
            .with_message(format!(
                "Traded {i_hand_over} {} for {they_hand_over} {}",
                mine.0, theirs.0
            ))
    }

    /// `Action::GoWithout`.
    pub(in crate::analytics) fn going_without(&mut self, for_them: &uuid::Uuid, agent_index: usize) -> ActionResult {
        // The other half of laying down your life for somebody, and
        // the half that happens more often than the fighting. A gift
        // is what a person can spare; this is what they cannot.
        let Some(them) = self
            .population
            .agents
            .iter()
            .position(|other| other.id == *for_them && other.state.is_alive)
        else {
            return ActionResult::failure("Nobody there to give to".to_string());
        };

        let Some(mine) = self.population.agents[agent_index]
            .find_best_food_to_eat()
            .filter(|what| self.population.agents[agent_index].how_many_i_have(what) > 0)
        else {
            return ActionResult::failure("Nothing to go without".to_string());
        };

        let me = self.population.agents[agent_index].id;

        {
            let agent = &mut self.population.agents[agent_index];
        }

        if self.hand_over(agent_index, them, &mine, 1) == 0 {
            return ActionResult::failure("No room in their pack for it".to_string());
        }

        {
            let other = &mut self.population.agents[them];
            other.they_did_me_a_good_turn(me, Self::WHAT_GOING_WITHOUT_IS_WORTH);
        }

        debug!("Agent {me} went without their own {mine} for {for_them}");

        ActionResult::success()
            .with_drive_change(DriveType::Protection, -0.5)
            .with_message(format!("Went without their own {mine}"))
    }

    /// `Action::GiveTo`.
    pub(in crate::analytics) fn giving_to(&mut self, to: &uuid::Uuid, agent_index: usize, tick_now: u32) -> ActionResult {
        let Some(them) = self
            .population
            .agents
            .iter()
            .position(|other| other.id == *to && other.state.is_alive)
        else {
            return ActionResult::failure("Nobody there to give to".to_string());
        };

        // Something they are short of that I have too much of. A gift
        // is one-sided: what the other has is nothing to do with it.
        let Some(mine) = self.what_i_would_hand_over(agent_index, them) else {
            return ActionResult::failure(
                "Nothing of mine they have any use for".to_string(),
            );
        };

        let handed_over = (mine.1 / 2).max(1);
        let me = self.population.agents[agent_index].id;

        let handed_over = self.hand_over(agent_index, them, &mine.0, handed_over);
        if handed_over == 0 {
            return ActionResult::failure("No room in their pack for it".to_string());
        }

        {
            let agent = &mut self.population.agents[agent_index];
            agent.skills.practise(crate::agents::SkillType::Social, 10, tick_now);
        }

        {
            let other = &mut self.population.agents[them];
            other.they_did_me_a_good_turn(me, Self::WHAT_A_GIFT_IS_WORTH);
        }

        debug!("Agent {me} gave away {handed_over} {}", mine.0);

        ActionResult::success()
            .with_drive_change(DriveType::Social, -0.4)
            .with_energy_cost(1.0)
            .with_message(format!("Gave away {handed_over} {}", mine.0))
    }
}
