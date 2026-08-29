// src/analytics/turn/each_one.rs
//! One person's turn, in the order a person takes it.
//!
//! Everybody in the settlement goes through these stages every tick, and until
//! this split they were six hundred and seventy lines in the middle of `tick`
//! with no names on them:
//!
//! 1. [`Simulation::keep_the_goals_and_the_plan_current`] - the standing
//!    intentions, refreshed on their own clock rather than every turn.
//! 2. [`Simulation::choose_what_to_do`] - the priority ladder, from starving
//!    down through fear, shelter, what can be seen, the plan, the goals and
//!    finally the drives; and the note of *why*, which is the only thing that
//!    makes the threat tally mean anything.
//! 3. [`Simulation::and_what_it_takes`] - what the chosen thing actually
//!    requires: a real target rather than a nil id, an errand held rather than
//!    re-decided, a reachable place, a free hand, the tool out of the bag, the
//!    parts fetched, and whether a better tool would pay for itself.
//! 4. `execute_action` - see [`crate::analytics::doing`].
//! 5. [`Simulation::what_came_of_it`] - the body's bill, the tally, the
//!    lesson, the plan's progress and the goals.
//!
//! Then, on its own clock rather than on a drive,
//! [`Simulation::look_in_at_the_storehouse`].

use super::super::Simulation;
use crate::core::DriveType;
use crate::environment::{Action, ActionResult};
use log::debug;

impl Simulation {
    /// Everybody takes a turn.
    ///
    /// By id rather than by index, and the index re-found each time, because a
    /// turn can kill somebody: an index taken before the loop would be pointing
    /// at the wrong person by the time it was used.
    pub(in crate::analytics) fn everybody_takes_a_turn(&mut self) {
        // Collected up front so that a social action can resolve a nil id
        // against who was actually standing about when the turn began.
        let agent_ids: Vec<_> = self.population.agents.iter().map(|a| a.id).collect();
        let agent_positions: Vec<(uuid::Uuid, (i32, i32, i32))> = self
            .population
            .agents
            .iter()
            .map(|a| (a.id, a.state.position))
            .collect();

        for agent_id in agent_ids {
            let Some(agent_index) = self.population.agents.iter().position(|a| a.id == agent_id)
            else {
                continue;
            };

            // What is pressing hardest, and where this one is standing.
            let (drive_type, drive_value, agent_position) = {
                let agent = &self.population.agents[agent_index];
                let drive = agent.drives.get_most_urgent();
                (
                    drive.map(|d| d.drive_type),
                    drive.map(|d| d.value).unwrap_or(0.0),
                    agent.state.position,
                )
            };
            let Some(drive_type) = drive_type else { continue };

            debug!(
                "Agent {} - Most urgent drive: {:?} (value: {:.2})",
                agent_id, drive_type, drive_value
            );

            // The tree is consulted for learning, not for permission. Gating
            // the pipeline on it meant an agent whose most urgent drive had no
            // tree - or whose tree had been pruned - stopped acting entirely
            // and stood still until it died.
            let (tree_name, execution_result) = {
                let agent = &mut self.population.agents[agent_index];
                match agent.select_behavior_tree() {
                    Some(tree) => {
                        let name = tree.name.clone();
                        (Some(name), Some(tree.execute()))
                    }
                    None => (None, None),
                }
            };
            if let (Some(name), Some(result)) = (tree_name.as_ref(), execution_result.as_ref()) {
                debug!(
                    "Agent {} executing behavior tree '{}': {:?}",
                    agent_id, name, result
                );
            }

            self.keep_the_goals_and_the_plan_current(agent_index);

            // Fleeing comes out as an ordinary `Move`, so without a note of why
            // it was chosen it is invisible to both the tally and the errand.
            let mut ran_for_it = false;
            let (action, is_plan_action) =
                self.choose_what_to_do(agent_index, agent_position, &mut ran_for_it);

            let action = self.and_what_it_takes(
                action,
                agent_index,
                agent_id,
                agent_position,
                &agent_positions,
                ran_for_it,
            );

            let action_result = self.execute_action(&action, agent_index);

            self.what_came_of_it(
                &action,
                &action_result,
                agent_index,
                agent_id,
                drive_type,
                is_plan_action,
            );

            self.look_in_at_the_storehouse(agent_index);
        }
    }

    /// The standing intentions: goals generated on their own clock, and a plan
    /// dropped when the world has already answered what it was for.
    fn keep_the_goals_and_the_plan_current(&mut self, agent_index: usize) {
        let agent_id = self.population.agents[agent_index].id;

        // Generate goals periodically based on drives and emotions
        if self.current_tick % 50 == 0 {
            let agent = &mut self.population.agents[agent_index];

            // Collect current drive types and emotion values
            let drive_types: Vec<crate::core::DriveType> = crate::core::DriveType::all().to_vec();
            let emotion_values: Vec<(crate::core::EmotionType, f32)> = vec![
                (crate::core::EmotionType::Happiness, agent.emotions.happiness),
                (crate::core::EmotionType::Fear, agent.emotions.fear),
                (crate::core::EmotionType::Anger, agent.emotions.anger),
                (crate::core::EmotionType::Sadness, agent.emotions.sadness),
                (crate::core::EmotionType::Curiosity, 0.5), // Default curiosity level
            ];

            // Generate common goals based on current state
            let new_goals = crate::core::goals::GoalManager::generate_common_goals(
                &drive_types,
                &emotion_values,
                self.current_tick,
            );

            // Add generated goals to agent's goals
            for goal in new_goals {
                agent.goals.add_goal(goal);
            }
        }

        // Check if current plan is still relevant given updated world state
        // This allows agents to abandon plans when goals are already satisfied
        // (e.g., another agent restocked the storehouse)
        {
            use crate::world::ItemType;
            use crate::core::GoalWorldState;

            // Calculate storehouse contents
            let food_types = vec![
                ItemType::Food, ItemType::Bread, ItemType::Cheese,
                ItemType::Meat, ItemType::Fish, ItemType::Honey, ItemType::Ale,
            ];
            let resource_types = vec![
                ItemType::Wood, ItemType::Stone, ItemType::Iron,
                ItemType::Clay, ItemType::Sand, ItemType::Coal,
            ];
            let tool_types = vec![
                ItemType::WoodenAxe, ItemType::StoneAxe, ItemType::IronAxe,
                ItemType::WoodenPickaxe, ItemType::StonePickaxe, ItemType::IronPickaxe,
                ItemType::WoodenHammer, ItemType::StoneHammer, ItemType::IronHammer,
            ];

            let storehouse_food: u32 = food_types.iter()
                .filter_map(|&item| self.world.storehouse_inventory.items.get(&item))
                .map(|item| item.quantity)
                .sum();

            let storehouse_materials: u32 = resource_types.iter()
                .filter_map(|&item| self.world.storehouse_inventory.items.get(&item))
                .map(|item| item.quantity)
                .sum();

            let storehouse_tools: u32 = tool_types.iter()
                .filter_map(|&item| self.world.storehouse_inventory.items.get(&item))
                .map(|item| item.quantity)
                .sum();

            // Get agent's personal inventory state
            let agent = &self.population.agents[agent_index];
            let personal_food = agent.inventory.get_item("food")
                .map(|i| i.quantity)
                .unwrap_or(0);
            let gathered_resources = agent.inventory.get_item("wood")
                .map(|i| i.quantity)
                .unwrap_or(0)
                + agent.inventory.get_item("stone")
                    .map(|i| i.quantity)
                    .unwrap_or(0);

            // Check if agent has protection equipment (check for any armor items)
            let has_protection = agent.inventory.get_all_items().iter()
                .any(|(item_id, _)| {
                    item_id.contains("armor") ||
                    item_id.contains("Armor") ||
                    item_id.contains("shield")
                });

            // Check if agent owns a house by checking actual building ownership
            let owns_house = self.world.buildings.iter().any(|b| {
                b.owner == Some(agent_id) &&
                b.is_completed() &&
                b.building_type.is_residential()
            });

            let world_state = GoalWorldState {
                storehouse_food,
                storehouse_materials,
                storehouse_tools,
                personal_food,
                gathered_resources,
                owns_house,
                has_protection,
                ..Default::default()
            };

            // Update plan relevance - this will abandon the plan if goal is satisfied
            let agent = &mut self.population.agents[agent_index];
            agent.update_plan_relevance(&world_state);
        }
    }

    /// What this one decides to do, and the note of why.
    ///
    /// The priority ladder, in order: starving, what was felt strongly enough
    /// to act on, shelter, what can be seen, the plan, the goals, the drives.
    /// `ran_for_it` is written out at the end because fleeing is an ordinary
    /// `Move` and the errand and the tally both need to know it was one.
    fn choose_what_to_do(
        &mut self,
        agent_index: usize,
        agent_position: (i32, i32, i32),
        ran_for_it: &mut bool,
    ) -> (Action, bool) {
        let agent_id = self.population.agents[agent_index].id;

        // Generate action based on priority:
        // starvation > emotions > shelter > percepts > plan > goals > drives
        //
        // Running away comes out as an ordinary Move, so without a
        // note of why it was chosen it is invisible in the tally
        let mut running_away = false;

        // What this one felt, and what the threat tree made of it.
        // Both are read out of the block below and tallied after it,
        // because everything in there holds `self` immutably.
        let felt: Option<&'static str>;
        let on_the_mind: Option<&'static str>;
        let under_the_gate: Option<&'static str>;
        let mut came_to: Option<&'static str> = None;

        let (action, is_plan_action) = {
            let agent = &self.population.agents[agent_index];

            felt = if agent.emotions.should_flee() {
                Some("felt: afraid enough to act")
            } else if agent.emotions.should_attack() {
                Some("felt: angry enough to act")
            } else {
                None
            };

            // And whether there was a creature on this one's mind at
            // all, at any strength. The gap between this and `felt` is
            // the gate: a man who is afraid of the wolf he can see but
            // not afraid *enough* never reaches the tree.
            on_the_mind = if agent.emotions.what_frightens_me_most().is_some() {
                Some("a creature is on the mind: feared")
            } else if agent.emotions.what_angers_me_most().is_some() {
                Some("a creature is on the mind: resented")
            } else {
                None
            };

            // And which half of the gate turned it away, for the
            // turns where something was on the mind and nothing came
            // of it. `should_attack` wants anger over a half *and*
            // fear under three tenths, so there are two ways to fail
            // it and they want different fixes.
            let (how_afraid, how_angry) = (agent.emotions.fear, agent.emotions.anger);
            under_the_gate = if how_angry > 0.5 {
                Some("under the gate: angry, but too frightened to stand")
            } else if how_afraid > 0.3 {
                Some("under the gate: uneasy, but not angry enough")
            } else {
                Some("under the gate: neither strongly enough")
            };

            // PRIORITY -1: An agent already starving eats before it does
            // anything else, including running from a threat.
            if let Some(survival_action) =
                self.survival_action(agent, agent_position, true)
            {
                debug!(
                    "Agent {} is starving at {:?}, survival action: {:?}",
                    agent_id, agent_position, survival_action
                );
                (survival_action, false)
            }
            // PRIORITY -0.6: an armful of the harvest, and somewhere
            // to put it.
            //
            // Above hunger on purpose, and only in autumn. A person
            // filling a store is a person carrying food past their
            // own mouth, and Hunger is a primary drive that wins
            // every contest it enters - so while this sat below it, a
            // load never survived long enough to reach the pit. The
            // starving branch above still beats it: a man who will be
            // dead by morning eats what is in his hand.
            else if let Some(carrying_home) =
                self.is_the_load_worth_carrying_home(agent, agent_position)
            {
                debug!("Agent {agent_id} is taking a load to the store");
                (carrying_home, false)
            }
            // PRIORITY -0.5: somebody of this agent's own who will
            // not last the week, and food in the pack it is going to
            // want itself.
            //
            // An override rather than a drive, and it has to be: a
            // sacrifice that only happens when nothing else is
            // pressing is not a sacrifice. The agent has already been
            // asked whether it is itself past bearing, above, and
            // `somebody_of_mine_who_needs_it_more` refuses again -
            // two dead people is not better than one.
            else if let Some(for_them) =
                self.somebody_of_mine_who_needs_it_more(agent, agent_position)
            {
                debug!("Agent {agent_id} is going without for {for_them}");
                (Action::GoWithout { for_them }, false)
            }
            // PRIORITY 0: Check emotional overrides (fear/anger from
            // what is in front of the agent, or from being attacked)
            else if agent.emotions.should_flee() {
                // Frightened of something that is actually there. The
                // whole tree lives in `what_this_threat_comes_to` - run,
                // or turn and fight if there is nowhere to run, or freeze
                // if there is neither. The attacker branches below it are
                // for agents, who are not creatures.
                let tree = self.what_this_threat_comes_to(agent, agent_position);
                came_to = Some(tree.0);

                if let Some(away) = tree.1.or_else(|| {
                    self.run_from_whoever_frightens_me(agent, agent_position)
                }) {
                    debug!(
                        "Agent {} RUNNING from {:?} (fear={:.2})",
                        agent_id,
                        agent.emotions.what_frightens_me_most().map(|(k, _)| k),
                        agent.emotions.fear
                    );
                    running_away = true;
                    (away, false)
                }
                // High fear - flee from attacker or danger
                else if let Some(attacker_id) = agent.emotions.recent_attacker(self.current_tick) {
                    // Find attacker position and flee away from them
                    if let Some(attacker) = self.population.agents.iter().find(|a| a.id == attacker_id) {
                        let attacker_pos = attacker.state.position;
                        let dx = agent_position.0 - attacker_pos.0;
                        let dy = agent_position.1 - attacker_pos.1;
                        let distance = ((dx * dx + dy * dy) as f32).sqrt().max(1.0);
                        let flee_distance = 15;
                        let flee_x = agent_position.0 + ((dx as f32 / distance) * flee_distance as f32) as i32;
                        let flee_y = agent_position.1 + ((dy as f32 / distance) * flee_distance as f32) as i32;

                        debug!(
                            "Agent {} FLEEING from attacker {} (fear={:.2})",
                            agent_id, attacker_id, agent.emotions.fear
                        );

                        (crate::environment::Action::Move {
                            target: (flee_x, flee_y, agent_position.2),
                        }, false)
                    } else {
                        // Attacker not found, flee in random direction
                        use rand::Rng;
                        let mut rng = crate::core::dice::roll();
                        let flee_x = agent_position.0 + rng.gen_range(-15..=15);
                        let flee_y = agent_position.1 + rng.gen_range(-15..=15);
                        (crate::environment::Action::Move {
                            target: (flee_x, flee_y, agent_position.2),
                        }, false)
                    }
                } else {
                    // No specific attacker, continue with other priorities
                    // (fear might be from other sources like predators)
                    self.generate_non_emotional_action(agent, agent_position)
                }
            } else if agent.emotions.should_attack() {
                // Angry at something within arm's reach: turn on it.
                // An angry agent stands its ground - it does not walk
                // across the map after a wolf it can see, which is
                // what keeps this from eating a settlement's whole day.
                let grudge = self.round_on_whoever_angers_me(agent, agent_position);
                came_to = Some(if grudge.is_some() {
                    "a grudge answered before the tree was asked"
                } else {
                    self.what_this_threat_comes_to(agent, agent_position).0
                });

                if let Some(strike) = grudge.or_else(|| {
                    self.what_this_threat_comes_to(agent, agent_position).1
                }) {
                    debug!(
                        "Agent {} STANDING GROUND against {:?} (anger={:.2})",
                        agent_id,
                        agent.emotions.what_angers_me_most().map(|(k, _)| k),
                        agent.emotions.anger
                    );
                    (strike, false)
                }
                // High anger, low fear - retaliate against attacker
                else if let Some(attacker_id) = agent.emotions.recent_attacker(self.current_tick) {
                    debug!(
                        "Agent {} RETALIATING against {} (anger={:.2}, fear={:.2})",
                        agent_id, attacker_id, agent.emotions.anger, agent.emotions.fear
                    );

                    (crate::environment::Action::Attack {
                        target_agent_id: attacker_id,
                        weapon: agent.equipment.get_weapon().map(|w| w.name.clone()),
                    }, false)
                } else {
                    // Angry but no target, continue with other priorities
                    self.generate_non_emotional_action(agent, agent_position)
                }
            } else {
                self.generate_non_emotional_action(agent, agent_position)
            }
        };

        // And now the block is done with `self`, book what the
        // feelings came to. See `what_a_threat_came_to`.
        let mut book = |what: &str| {
            *self
                .what_a_threat_came_to
                .entry(what.to_string())
                .or_insert(0) += 1;
        };

        book("turns decided");
        if let Some(on_the_mind) = on_the_mind {
            book(on_the_mind);
        }
        if let Some(felt) = felt {
            book(felt);
            if came_to.is_none() {
                // Frightened enough to act on, and something further
                // up the priority list went first
                book("something else came first");
            }
        } else if on_the_mind.is_some() {
            book("on the mind, but under the gate");
            if let Some(under_the_gate) = under_the_gate {
                book(under_the_gate);
            }
        }
        if let Some(came_to) = came_to {
            book(came_to);
        }
        *ran_for_it = running_away;
        (action, is_plan_action)
    }

    /// What the chosen thing actually takes, before anybody tries it.
    fn and_what_it_takes(
        &mut self,
        action: Action,
        agent_index: usize,
        agent_id: uuid::Uuid,
        agent_position: (i32, i32, i32),
        agent_positions: &[(uuid::Uuid, (i32, i32, i32))],
        running_away: bool,
    ) -> Action {
        // Resolve nil UUIDs in social actions to actual nearby agents
        let action = Self::resolve_action_target(
            action,
            agent_id,
            agent_position,
            &agent_positions,
        );

        // An errand held across turns rather than re-decided at every
        // step. This is what makes a walk to the river something an
        // agent can finish - see `Errand`.
        let action = self.stick_to_the_errand(agent_index, action, running_away);

        // Drop travel plans toward places the agent cannot reach
        let action = self.retarget_unreachable_move(agent_index, action);

        // A job that wants a hand free, and both of them full. A
        // person does not stand there defeated by it: they put
        // something down and get on with it. This is the only place
        // `Unequip` is ever chosen, because it is the only reason
        // anybody would.
        let action = self.free_a_hand_for(action, agent_index);

        // And a job whose tool is still in the bag: spend the turn
        // getting it out. The first cut of this put equipping at the
        // bottom of the Utility chain, where it fired half a time in
        // a world of ten thousand ticks - there is always some
        // material wanting fetching, so nothing ever reached it.
        // Reaching for a tool is not what somebody does with a spare
        // moment, it is what they do just before using it.
        let action = self.get_the_tool_out_for(action, agent_index);

        // And a job whose tool this one has not got at all, but knows
        // how to make. The turn was going to be a refusal; it goes on
        // the tool instead.
        let action = self.make_what_this_wants(action, agent_index);

        // And a job this one *could* do now, but would do faster with
        // a tool worth stopping for. The turn was going to be work; it
        // goes on the tool because the tool buys back more work than
        // it costs. See `would_a_better_tool_pay`.
        let action = self.would_a_better_tool_pay(action, agent_index);

        // What the settlement spends its days doing
        *self
            .actions_taken
            .entry(Self::what_to_book(&action, running_away))
            .or_insert(0) += 1;
        action
    }

    /// What came of it: the body's bill, the tally, the lesson, and the plan.
    fn what_came_of_it(
        &mut self,
        action: &Action,
        action_result: &ActionResult,
        agent_index: usize,
        agent_id: uuid::Uuid,
        drive_type: DriveType,
        is_plan_action: bool,
    ) {
        // What the turn's work cost is what the body burned doing it.
        // A body walking and digging burns and sweats faster than one
        // asleep, which is the whole of "increased physical activity
        // should increase the rate at which hunger and thirst
        // increase". The action matrix already prices this.
        self.population.agents[agent_index].state.effort_this_turn +=
            action_result.energy_cost;

        if !action_result.success {
            *self
                .actions_failed
                .entry(Self::name_of(&action))
                .or_insert(0) += 1;
            if let Some(why) = action_result.message.as_ref() {
                *self
                    .actions_failed_because
                    .entry(format!("{}: {}", Self::name_of(&action), why))
                    .or_insert(0) += 1;
            }
        }

        debug!(
            "Agent {} - Action result: {} (satisfaction: {:.2})",
            agent_id,
            action_result.message.as_ref().map(|s| s.as_str()).unwrap_or("No message"),
            action_result.drive_satisfaction
        );

        // Broadcast action to nearby observers (for observational learning)
        if action_result.success {
            let agent = &self.population.agents[agent_index];
            let agent_pos = agent.state.position;

            // Map action to ActionType for broadcasting
            if let Some(broadcast_type) = Self::map_action_to_broadcast_type(&action) {
                self.population.broadcast_action(
                    agent_id,
                    agent_pos,
                    broadcast_type,
                    true, // success
                    format!("{:?}", action),
                    self.current_tick as u64,
                );
            }
        }

        // What the world was doing while that was attempted, taken
        // before the agent is borrowed to be told about it
        let what_it_was_like = {
            let agent = &self.population.agents[agent_index];
            self.what_it_is_like_here(agent, agent.state.position)
        };

        // Apply feedback to agent (drive satisfaction)
        let agent = &mut self.population.agents[agent_index];
        agent.apply_feedback(&action_result, drive_type);

        // And note how it went, so the agent does more of what pays
        // and less of what does not - and note what the afternoon was
        // like, so it can work out for itself which afternoons pay
        agent.learn_from_this_here(&action, action_result.success, &what_it_was_like);

        // Then join the doing to the need it answered and the ground
        // it was answered on, which is what lets a thirsty man walk
        // back to the bank he drank from yesterday
        let where_it_was = agent.state.position;
        let now = self.current_tick;
        agent.link_what_worked(&action, &action_result, drive_type, where_it_was, now);

        // Apply trait-based happiness rewards for successful actions
        if action_result.success {
            agent.apply_trait_action_rewards(&action);
        }

        // Update plan execution state if this was a plan action
        if is_plan_action {
            if action_result.success {
                // Successful action - advance to next plan step
                agent.advance_plan_step(true, agent.plan_step_ticks + 1);
                debug!(
                    "Agent {} completed plan step, progress: {:?}",
                    agent_id,
                    agent.plan_progress()
                );
            } else {
                // Failed action - increment step ticks and potentially abandon plan
                agent.tick_plan_step();
                if !agent.should_execute_plan() {
                    // Plan has timed out or is no longer viable
                    debug!("Agent {} abandoning plan due to failure/timeout", agent_id);
                    agent.abandon_plan();
                }
            }
        } else {
            // Not a plan action - tick the plan step counter anyway
            // This allows plans to timeout if agent keeps getting interrupted
            agent.tick_plan_step();
        }

        // Try to create a plan for goals if agent doesn't have one
        // Only do this periodically to avoid constant replanning
        if !agent.has_active_plan() && self.current_tick % 50 == 0 {
            // Use a default resource/return location (should be enhanced with real world data)
            let resource_loc = (50, 50, 0);
            let return_loc = (0, 0, 0);
            if agent.create_plan_for_goal(resource_loc, return_loc, self.current_tick) {
                debug!(
                    "Agent {} created new plan: {:?}",
                    agent_id,
                    agent.current_plan_step_description()
                );
            }
        }

        // Update behavior tree weights based on action success
        if let Some(tree) = agent.select_behavior_tree() {
            if action_result.success {
                tree.total_successes += 1;
            }
        }

        // Update goal progress based on action result
        if action_result.success {
            let action_name = format!("{:?}", action);
            let agent = &mut self.population.agents[agent_index];

            // Check if action aligns with any active goals and update progress
            if agent.goals.action_aligns_with_goals(&action_name) {
                // Find the highest priority goal that aligns with this action
                if let Some(goal) = agent.goals.highest_priority_goal() {
                    let progress_delta = match &action {
                        // Resource gathering actions
                        crate::environment::Action::Gather { .. } => 0.2,
                        crate::environment::Action::Hunt { .. } => 0.15,

                        // Building and crafting
                        crate::environment::Action::Build { .. } => 0.3,
                        crate::environment::Action::Craft { .. } => 0.25,

                        // Social actions
                        crate::environment::Action::Mate { .. } => 0.2,
                        crate::environment::Action::Socialize { .. } => 0.15,

                        // Emotional satisfaction
                        crate::environment::Action::Sleep { .. } => 0.1,
                        crate::environment::Action::Eat { .. } => 0.1,

                        _ => 0.05, // Small progress for other actions
                    };

                    let goal_id = goal.id;
                    agent.goals.update_goal_progress(goal_id, progress_delta);
                }
            }
        }

        // Cleanup completed goals periodically
        if self.current_tick % 100 == 0 {
            let agent = &mut self.population.agents[agent_index];
            agent.goals.cleanup_completed();
        }
    }

    /// Looking in at the storehouse, on its own clock rather than on a drive.
    ///
    /// Deliberately outside the drive pipeline: this is what makes a store a
    /// shared thing rather than one man's pack.
    fn look_in_at_the_storehouse(&mut self, agent_index: usize) {
        let agent_id = self.population.agents[agent_index].id;

        // Check if agent should interact with storehouse (every 20 ticks, or when Preparedness is high)
        // This happens independently of drive-based actions to enable cooperative resource sharing
        if self.current_tick % 20 == 0 || {
            let agent = &self.population.agents[agent_index];
            agent.drives.get(DriveType::Preparedness)
                .map(|d| d.value > 0.6)
                .unwrap_or(false)
        } {
            // Calculate storehouse contents
            let (storehouse_food, storehouse_resources) = {
                use crate::world::ItemType;
                

                let food_types = vec![
                    ItemType::Food, ItemType::Bread, ItemType::Cheese,
                    ItemType::Meat, ItemType::Fish, ItemType::Honey, ItemType::Ale,
                ];
                let resource_types = vec![
                    ItemType::Wood, ItemType::Stone, ItemType::Iron,
                    ItemType::Clay, ItemType::Sand, ItemType::Coal,
                ];

                let food_total: u32 = food_types.iter()
                    .filter_map(|&item| self.world.storehouse_inventory.items.get(&item))
                    .map(|item| item.quantity)
                    .sum();

                let resource_total: u32 = resource_types.iter()
                    .filter_map(|&item| self.world.storehouse_inventory.items.get(&item))
                    .map(|item| item.quantity)
                    .sum();

                (food_total, resource_total)
            };

            // Get storage action from agent
            let storage_action = {
                let agent = &self.population.agents[agent_index];
                agent.decide_storage_action(storehouse_food, storehouse_resources)
            };

            // Execute storage action if one was decided
            if let Some(action) = storage_action {
                debug!("Agent {} performing storage action: {:?}", agent_id, action);
                let action_result = self.execute_action(&action, agent_index);

                debug!(
                    "Agent {} - Storage action result: {}",
                    agent_id,
                    action_result.message.as_ref().map(|s| s.as_str()).unwrap_or("No message")
                );
            }
        }
    }
}
