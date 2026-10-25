// src/analytics/turn/each_one.rs
//! One person's turn, in the order a person takes it.
//!
//! Everybody in the settlement goes through these stages every turn, and until
//! this split they were six hundred and seventy lines in the middle of `turn`
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
    /// Ready the half hour's deciding: who stood where as it began, and
    /// everybody's own half hour begun at their own minute.
    pub(in crate::analytics) fn everybody_takes_a_turn(&mut self) {
        // Collected up front so that a social action can resolve a nil id
        // against who was actually standing about when the turn began.
        self.where_everybody_stood = self
            .population
            .agents
            .iter()
            .map(|a| (a.id, a.state.position))
            .collect();

        // **Everybody keeps their own half hour.** "Agents should have their
        // 30 minute decision ticks separated out to prevent lag spikes if
        // every agent was forced to make decisions at the exact same time."
        // Each person's half hour begins at their own minute of it, fixed by
        // who they are, so the deciding is spread over the thirty minutes
        // rather than all of it falling at once. See `take_a_minute`.
        let began = self.current_turn;
        for agent in self.population.agents.iter_mut() {
            if agent.state.next_decision_at == 0 {
                agent.state.next_decision_at = began + Self::my_minute_of_the_half_hour(agent.id);
            }
        }
    }

    /// One minute of the half hour: whoever's decision falls on it decides,
    /// and whatever is after somebody taking things a minute at a time gets
    /// its minute at them.
    pub(in crate::analytics) fn this_minute(
        &mut self,
        now: u32,
        agent_positions: &[(uuid::Uuid, (i32, i32, i32))],
    ) {
        let due: Vec<uuid::Uuid> = self
            .population
            .agents
            .iter()
            .filter(|a| a.state.is_alive && a.state.next_decision_at <= now)
            .map(|a| a.id)
            .collect();
        for agent_id in due {
            self.the_next_decision_kept_apart(agent_id, now, agent_positions);
        }

        // "The minute-to-minute decision making was meant to occur when a
        // threat was within eyesight of an agent." Somebody taking things a
        // minute at a time is somebody with something after them, and the
        // thing after them gets its minutes too, or running away would be
        // free: a man who gets out of reach in the first two minutes is not
        // struck at in the other twenty-eight, and one who is cornered is
        // struck at every minute he stays there.
        let watching: Vec<usize> = self
            .population
            .agents
            .iter()
            .enumerate()
            .filter(|(_, a)| a.state.is_alive && a.state.deciding_by_the_minute)
            .map(|(at, _)| at)
            .collect();
        for at in watching {
            self.minutes_spent_in_danger += 1;
            self.predators_try_their_luck_at(at);
        }
    }

    /// Which minute of the half hour this one's own half hour begins on.
    pub(in crate::analytics) fn my_minute_of_the_half_hour(id: uuid::Uuid) -> u32 {
        (id.as_u128() % crate::environment::seasons::MINUTES_PER_TURN as u128) as u32
    }

    /// One person decides, and how long it is until they decide again.
    ///
    /// Half an hour at a time, with every need on the table: hunger, thirst,
    /// the lot. A minute at a time only while something that menaces them is
    /// in sight, and those minutes come out of their own half hour rather than
    /// being added to it - so somebody walking to the stream who comes into
    /// sight of a wolf stops where they are, and takes what is left of the
    /// half hour a minute at a time.
    /// One person's turn, kept apart from everybody else's.
    ///
    /// A fault in one person's turn used to end the whole run and lose
    /// everything since the last save; a probability slip in conception did
    /// that to one run in twenty-five while it stood (#4). Outside the tests a
    /// turn that panics is caught, written to the log and counted, and that
    /// person loses their half hour; everybody else goes on. Under test it is
    /// left to fail loudly, because a test is where a fault should be seen.
    fn the_next_decision_kept_apart(
        &mut self,
        agent_id: uuid::Uuid,
        now: u32,
        agent_positions: &[(uuid::Uuid, (i32, i32, i32))],
    ) {
        #[cfg(test)]
        self.the_next_decision(agent_id, now, agent_positions);

        #[cfg(not(test))]
        {
            let went = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                self.the_next_decision(agent_id, now, agent_positions)
            }));
            if went.is_err() {
                self.turns_lost_to_a_fault += 1;
                log::error!("a turn of {agent_id} at {now} failed and was skipped");
                if let Some(agent) = self.population.agents.iter_mut().find(|a| a.id == agent_id) {
                    agent.state.next_decision_at = now + crate::environment::seasons::MINUTES_PER_TURN;
                }
            }
        }
    }

    fn the_next_decision(
        &mut self,
        agent_id: uuid::Uuid,
        now: u32,
        agent_positions: &[(uuid::Uuid, (i32, i32, i32))],
    ) {
        let Some(index) = self.population.agents.iter().position(|a| a.id == agent_id) else {
            return;
        };

        self.feel_again_about_what_is_round(index);
        let watching = self.something_is_after_me(index);
        let budget = if watching { 1 } else { crate::environment::seasons::MINUTES_PER_TURN };
        {
            let state = &mut self.population.agents[index].state;
            state.minutes_left_this_turn = budget as f32;
            state.walked_with_time_to_spare = false;
            state.deciding_by_the_minute = watching;
        }

        self.one_persons_turn(agent_id, agent_positions);

        // A walk that arrived with time to spare leaves the rest of the
        // budget for whatever is done on arriving.
        //
        // "An agent should easily be capable of walking a kilometer in
        // half an hour while accomplishing other tasks." A kilometre is
        // twelve minutes at five kilometres an hour, and somebody who has
        // walked to the larder has come to take something out of it, not
        // to stand there until the next half hour. So they decide again,
        // on what is about them now, until something that is not a walk
        // takes the rest of the time, or it has gone, or something has come
        // into sight that wants the rest of it taken a minute at a time.
        // Bounded, so that a run of short walks cannot go on for ever.
        let mut again = 0;
        let mut stopped_short = false;
        loop {
            let Some(index) = self.population.agents.iter().position(|a| a.id == agent_id) else {
                return;
            };
            let agent = &self.population.agents[index];
            if !agent.state.is_alive
                || !agent.state.walked_with_time_to_spare
                || agent.state.minutes_left_this_turn < Self::TIME_ENOUGH_TO_DO_SOMETHING
                || again >= Self::AS_MANY_THINGS_AS_A_HALF_HOUR_HOLDS
            {
                break;
            }
            if !watching {
                self.feel_again_about_what_is_round(index);
                if self.something_is_after_me(index) {
                    stopped_short = true;
                    break;
                }
            }
            let agent = &mut self.population.agents[index];
            agent.state.walked_with_time_to_spare = false;
            agent.busy_until = 0;
            again += 1;
            self.one_persons_turn(agent_id, agent_positions);
        }

        let Some(index) = self.population.agents.iter().position(|a| a.id == agent_id) else {
            return;
        };
        let state = &mut self.population.agents[index].state;
        let used = if stopped_short {
            (budget as f32 - state.minutes_left_this_turn).ceil().max(1.0) as u32
        } else {
            budget
        };
        state.next_decision_at = now + used;
        state.minutes_left_this_turn = 0.0;
        state.walked_with_time_to_spare = false;
    }

    /// How many minutes the beast after this one would take to kill them, at
    /// a blow a minute; `None` with nothing after them.
    pub(in crate::analytics) fn minutes_before_what_is_after_me_kills_me(&self, at: usize) -> Option<f32> {
        if !self.something_is_after_me(at) {
            return None;
        }
        let agent = &self.population.agents[at];
        let (x, y, _) = agent.state.position;
        let grown = agent.state.years_old() >= Self::TOO_BIG_FOR_A_SMALL_HUNTER;
        let worst_blow = self
            .what_menaces
            .iter()
            .filter(|menace| !(grown && menace.only_takes_the_small))
            .filter(|menace| {
                (menace.at.0 - x).abs().max((menace.at.1 - y).abs()) <= Self::CLOSE_ENOUGH_TO_WORRY_ABOUT
            })
            .map(|menace| agent.what_a_blow_costs_me(menace.damage))
            .fold(0.0_f32, f32::max);
        // Something after them that is no beast is a person, whose blows are
        // nearer a beast's than nothing
        let blow = if worst_blow > 0.0 { worst_blow } else { Self::WHAT_A_PERSONS_BLOW_COSTS };
        Some(agent.state.health.max(0.0) / blow)
    }

    /// What a blow from a person costs, for reckoning how soon one would kill.
    const WHAT_A_PERSONS_BLOW_COSTS: f32 = 10.0;

    /// How many minutes before hunger or thirst would kill this one.
    pub(in crate::analytics) fn minutes_before_want_kills_me(agent: &crate::agents::Agent) -> f32 {
        [crate::core::DriveType::Thirst, crate::core::DriveType::Hunger]
            .into_iter()
            .filter_map(|need| agent.state.turns_before_this_kills_me(need))
            .fold(f32::INFINITY, f32::min)
            * crate::environment::seasons::MINUTES_PER_TURN as f32
    }

    /// Whether something that menaces this one is in sight: a beast or a
    /// person they are afraid of, or angry enough at to face.
    ///
    /// Not hunger or thirst, which are answered on the half hour like every
    /// other need. And not for a small child, who takes no turn of its own:
    /// it is carried, and the minutes were only ever the beast's (#325).
    pub(in crate::analytics) fn something_is_after_me(&self, at: usize) -> bool {
        let agent = &self.population.agents[at];
        agent.state.years_old() >= crate::agents::LifeStage::KEPT_WITH_A_PARENT_UNTIL
            && agent.emotions.threatened_by_any_of(|source| self.is_it_about(at, source))
    }

    /// Whether what an agent fears or is angry at is about: a person only
    /// while they are within sight, anything else as the feeling says. A
    /// beast's hold on somebody fades once it is out of view; a grudge
    /// against a neighbour is held for days, and the minute clock is for
    /// something in front of you (#347).
    pub(in crate::analytics) fn is_it_about(&self, at: usize, source: &crate::agents::EmotionSource) -> bool {
        let crate::agents::EmotionSource::Agent(them) = source else {
            return true;
        };
        let here = self.population.agents[at].state.position;
        self.population.agents.iter().any(|other| {
            other.id == *them
                && other.state.is_alive
                && (other.state.position.0 - here.0).abs().max((other.state.position.1 - here.1).abs())
                    <= Self::WITHIN_SIGHT
        })
    }

    /// Fewer minutes than this left of the half hour is not enough to start
    /// anything on.
    pub(in crate::analytics) const TIME_ENOUGH_TO_DO_SOMETHING: f32 = 2.0;

    /// And how many decisions one half hour can hold.
    pub(in crate::analytics) const AS_MANY_THINGS_AS_A_HALF_HOUR_HOLDS: u32 = 6;

    /// One person's turn: what is pressing, what they do about it, and what
    /// came of it.
    ///
    /// Taken once a turn by everybody, and then once a minute by anybody in
    /// danger - see `everybody_takes_a_turn`.
    fn one_persons_turn(
        &mut self,
        agent_id: uuid::Uuid,
        agent_positions: &[(uuid::Uuid, (i32, i32, i32))],
    ) {
        {
            let Some(agent_index) = self.population.agents.iter().position(|a| a.id == agent_id)
            else {
                return;
            };

            // The small ones do not take a turn at all.
            //
            // "Age 0-2: ... This age skips decision/planning periods which
            // occur 48 times a day. Age 2-5: ... This age skips
            // decision/planning periods."
            //
            // Which is what a body that small does: it is carried, it is fed
            // out of whoever carries it - see `feed_the_small_children` - and
            // it is put where that person is by
            // `the_small_stay_with_their_people`. Everything a turn does from
            // here down is choosing and doing, and a child under six does
            // neither. Letting one through meant a two-year-old walking off
            // after whatever its drives said, which is the other half of why
            // only 41.9% of them had a parent in reach when it came to
            // feeding them.
            if self.population.agents[agent_index].state.years_old()
                < crate::agents::LifeStage::KEPT_WITH_A_PARENT_UNTIL
            {
                return;
            }

            // Somebody in the middle of something does not stop to think.
            //
            // "Once an agent plans an action, it would not change its mind
            // unless its situation changed in some manner." A planning period
            // is a gate on deciding, not a unit of doing: an agent who set out
            // on something reckoned at 145 ticks walks past the gates at 30,
            // 60, 90 and 120 and thinks again at 150. See `Agent::busy_until`
            // for what that quantising costs and buys.
            //
            // Danger is the carve-out the specification makes, and it takes
            // the plan with it: "This does not apply if an agent encounters a
            // dangerous situation, as they must then make decisions minute by
            // minute." A man who has just seen a wolf is no longer walking to
            // the hedgerow, so the undertaking is abandoned rather than merely
            // interrupted - otherwise he would go back to it the moment the
            // wolf left, having learnt nothing from the last half hour. The
            // minute-by-minute loop in `everybody_takes_a_turn` reaches this
            // function again on the same tick, and has to find him free.
            {
                let now = self.current_turn;
                let in_danger = self.population.agents[agent_index]
                    .emotions
                    .in_danger_from(|source| self.is_it_about(agent_index, source));
                let agent = &mut self.population.agents[agent_index];
                if in_danger {
                    agent.busy_until = 0;
                } else if agent.busy_until > now {
                    return;
                }

                // Free to decide, so awake: the night is over, or something
                // has broken it (#310).
                if agent.fatigue.is_sleeping {
                    agent.wake_up(now);
                }
            }

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
            let Some(drive_type) = drive_type else { return };

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

            // Whatever far-off place this one might set out for is read as it
            // stands today, not as it was when nobody was near it.
            self.wake_what_this_one_remembers(agent_index);

            // Fleeing comes out as an ordinary `Move`, so without a note of why
            // it was chosen it is invisible to both the tally and the errand.
            let mut ran_for_it = false;
            self.camp_move_proposed.borrow_mut().take();
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

            // How this turn's need gets answered is said by whatever answers
            // it, this turn - see `Agent::by_what_way`.
            self.population.agents[agent_index].by_what_way = None;
            let action_result = self.execute_action(&action, agent_index);

            // A move of camp proposed while deciding is carried out only if
            // this is the walk it proposed: anything that overruled it (the
            // night, an errand, danger) leaves the camp where it is (#314).
            let proposed = self.camp_move_proposed.borrow_mut().take();
            if let (Some(decided), crate::environment::Action::Move { target }) = (proposed, &action) {
                if decided.who == agent_id && (target.0, target.1) == decided.to {
                    self.the_camp_moves(decided);
                }
            }

            // And now he is busy with it until it is done.
            //
            // Set from what the verb matrix says the action costs - see
            // `AgentState::how_long_this_takes` - so that a longer undertaking
            // holds him past more gates. Anything of a single period leaves
            // him free at the next one, which is what everything cost before
            // this existed and what most things still cost.
            //
            // Or from what the doing of it said, where that was longer: a
            // making takes an unpractised hand more than one period.
            {
                let takes = crate::agents::AgentState::how_long_this_takes(&action).max(
                    action_result.took_periods * crate::environment::seasons::TICKS_BETWEEN_PLANS,
                );
                let now = self.current_turn;
                self.population.agents[agent_index].busy_until = now + takes;
            }

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
        if self.current_turn % crate::environment::seasons::ONCE_EVERY_FEW_DAYS == 0 {
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
                self.current_turn,
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
                b.owner() == Some(agent_id) &&
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

            // "Primary Drive demand should be determined by whichever drive
            // results in the largest threat to the continued survival of
            // the agent." A beast in sight that is after this one kills in
            // minutes, and want in hours or days: whichever would kill
            // first is answered first. Everything below that is not the
            // answer to the beast - eating, carrying a load home, going
            // without for a child - waits while it is there.
            let the_beast_first = self
                .minutes_before_what_is_after_me_kills_me(agent_index)
                .is_some_and(|beast| beast < Self::minutes_before_want_kills_me(agent));

            // PRIORITY -1: An agent already starving eats before it does
            // anything else - unless the thing after it would kill it
            // sooner than the hunger or the thirst.
            if let Some(survival_action) = (!the_beast_first)
                .then(|| self.survival_action(agent, agent_position, true))
                .flatten()
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
            else if let Some(carrying_home) = (!the_beast_first)
                .then(|| self.is_the_load_worth_carrying_home(agent, agent_position))
                .flatten()
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
            else if let Some(for_them) = (!the_beast_first)
                .then(|| self.somebody_of_mine_who_needs_it_more(agent, agent_position))
                .flatten()
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
                // `whoever_struck_me`, not `recent_attacker`: this branch looks
                // the striker up in the agent list, and what used to be stored
                // was as often as not an animal's uuid, which is found in no
                // agent list - so a man bitten by a wolf fell through to the
                // arm below and **fled in a random direction** rather than
                // away from the wolf. Something present is handled above, by
                // the threat tree, which reads creatures properly. See #212.
                else if let Some(attacker_id) = agent.emotions.whoever_struck_me(self.current_turn) {
                    // Find attacker position and flee away from them
                    if let Some(attacker) = self.population.agents.iter().find(|a| a.id == attacker_id) {
                        let attacker_pos = attacker.state.position;

                        debug!(
                            "Agent {} FLEEING from attacker {} (fear={:.2})",
                            agent_id, attacker_id, agent.emotions.fear
                        );

                        // Away from them, and from somebody standing on the
                        // same ground too: worked out here as a straight line
                        // away from where they stood, that was a walk to where
                        // this one already was, and an errand to it.
                        (self.away_on_this_side(
                            agent,
                            agent_position,
                            (attacker_pos.0, attacker_pos.1),
                            Self::put_ground_between(agent_position, (attacker_pos.0, attacker_pos.1)),
                        ), false)
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
                // And likewise here, where it was worse: this aimed an
                // `Attack` at the stored uuid, so a man mauled by a bear
                // swung at a person who does not exist - **2,185 refusals of
                // "Attack: Target agent not found"**. Standing up to the
                // animal itself is the branch above.
                // And not again today, if this bout is already fought (#296).
                else if let Some(attacker_id) = agent
                    .emotions
                    .whoever_struck_me(self.current_turn)
                    .filter(|&them| !agent.emotions.have_we_had_it_out(them, self.current_turn))
                {
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

        // A wound bleeding, one's own or somebody's near, before anything
        // but running for it - the night included (#351).
        let action = self.what_a_wound_asks(agent_index, action, running_away);

        // A body craving salt goes for it, or keeps to the journey it set
        // out on for it (#356). The night still has its say after.
        let action = self.what_the_salt_asks(agent_index, action, running_away);

        // And at night, sleep (#299).
        let action = self.what_the_night_asks(agent_index, action, running_away);

        // And a body with nothing left in it rests, night or day (#366).
        let action = self.what_a_spent_body_does(agent_index, action, running_away);

        // Drop travel plans toward places the agent cannot reach
        let action = self.retarget_unreachable_move(agent_index, action);

        // And a walk past the water starts with the vessel filled (#351)
        let action = self.water_for_the_walk(agent_index, action, running_away);

        // A job that wants a hand free, and both of them full. A
        // person does not stand there defeated by it: they put
        // something down and get on with it. This is the only place
        // `Unequip` is ever chosen, because it is the only reason
        // anybody would.
        let action = self.free_a_hand_for(action, agent_index);

        // And a job whose tool is still in the bag: spend the turn
        // getting it out. The first cut of this put equipping at the
        // bottom of the Utility chain, where it fired half a time in
        // a world of ten thousand turns - there is always some
        // material wanting fetching, so nothing ever reached it.
        // Reaching for a tool is not what somebody does with a spare
        // moment, it is what they do just before using it.
        let action = self.get_the_tool_out_for(action, agent_index);

        // And a job whose tool this one has not got at all, but knows
        // how to make. The turn was going to be a refusal; it goes on
        // the tool instead.
        let action = self.make_what_this_wants(action, agent_index);

        // And a making somebody simply decided on: hold on to it, the
        // way the two diversions above already do. This is the last
        // path into `Errand::to_make` and it was the one missing, so
        // the only makings anybody ever finished were the ones they
        // were pushed into. See `hold_on_to_the_making`.
        let action = self.hold_on_to_the_making(action, agent_index);

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

        // A refusal is counted; a spoiled attempt is not. The work began and
        // went wrong, which costs the turn and the makings - see
        // `ActionResult::attempted`.
        if !action_result.success && !action_result.attempted {
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
                    self.current_turn as u64,
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
        // Spoiling the makings teaches that this hand wants practice, which
        // the skill has already been given. What it must not teach is that
        // the undertaking does not work: a beginner who concludes that never
        // practises into a master, which is the whole point of a skill
        // deciding the odds.
        if action_result.success || !action_result.attempted {
            agent.learn_from_this_here(&action, action_result.success, &what_it_was_like);
        }

        // Then join the doing to the need it answered and the ground
        // it was answered on, which is what lets a thirsty man walk
        // back to the bank he drank from yesterday
        let where_it_was = agent.state.position;
        let now = self.current_turn;
        agent.link_what_worked(&action, &action_result, drive_type, where_it_was, now);

        // And this act joins the run, *after* the linking, so that a run is
        // always what led up to the thing being credited rather than
        // including it. See `Agent::what_led_up_to_this`.
        agent.that_is_what_i_just_did(&action);

        // Apply trait-based happiness rewards for successful actions
        if action_result.success {
            agent.apply_trait_action_rewards(&action);
        }

        // Update plan execution state if this was a plan action
        if is_plan_action {
            if action_result.success {
                // Successful action - advance to next plan step
                agent.advance_plan_step(true, agent.plan_step_turns + 1);
                debug!(
                    "Agent {} completed plan step, progress: {:?}",
                    agent_id,
                    agent.plan_progress()
                );
            } else {
                // Failed action - increment step turns and potentially abandon plan
                agent.turn_plan_step();
                if !agent.should_execute_plan() {
                    // Plan has timed out or is no longer viable
                    debug!("Agent {} abandoning plan due to failure/timeout", agent_id);
                    agent.abandon_plan();
                }
            }
        }

        // **And a turn that was not spent on the plan does not count against
        // it.** This used to turn the step counter anyway, on the reasoning
        // that a plan should time out if the agent keeps being interrupted -
        // and it locks the branch shut, because the counter is the only thing
        // `should_execute_plan` measures staleness by. A plan the ladder never
        // reaches ages out of its step's allowance, and once it has, it is
        // "stuck" and can never be reached again. Measured over twelve worlds:
        // **98.5% of bodies hold a plan and 1.3% would run one**, with only
        // 9.5% hungry or thirsty enough for the survival gate to be what
        // stopped them. The timeout ate the other eighty-eight points.
        //
        // A plan that has not been worked is not stuck on a step. It is
        // waiting, and how long it has been waiting is what `created_at`
        // already records.
        //
        // See ISSUES_FOUND.md #187.

        // A learned run advances when its step is actually taken.
        //
        // `is_plan_action` is the old planner's flag and is set where the
        // ladder falls through to it; a learned run comes back through the
        // drive's own answer, so what marks it as a plan step is that the
        // verb matches what the plan wanted. Advancing on the verb rather
        // than on a flag also means a step the agent would have taken anyway
        // still counts, which is right: the plan is a claim about order, not
        // about who chose it.
        {
            let taken = crate::agents::Agent::what_was_tried(&action);
            let verb = crate::agents::Agent::just_the_verb(&taken);
            let agent = &mut self.population.agents[agent_index];
            if let Some(wanted) = agent.what_the_plan_wants_next() {
                if wanted == verb {
                    if action_result.success {
                        agent.advance_plan_step(true, agent.plan_step_turns + 1);
                    } else {
                        agent.turn_plan_step();
                    }
                } else {
                    // A turn spent on something else is a turn the plan is
                    // waiting through, and enough of them mean it is not the
                    // plan any more.
                    agent.turn_plan_step();
                }
                if !agent.should_execute_plan() && agent.has_active_plan() {
                    agent.abandon_plan();
                }
            }
        }

        let agent = &mut self.population.agents[agent_index];

        // The run this one has worked out for what is pressing, laid down as
        // a plan it can hold across turns.
        //
        // This is what the plan branch is for now. It is offered first
        // because a chain out of `Patterns` is a plan *for a need*, made of
        // verbs the model acts in and in an order the agent found out; the
        // goal planner below is the old one, whose steps are written against
        // hard-coded coordinates. See `Agent::plan_the_run_that_answers`.
        // It may displace a goal plan, which is not being executed anyway -
        // see the gate in `should_execute_plan`.
        if let Some(pressing) = agent.drives.get_most_urgent().map(|drive| drive.drive_type) {
            agent.plan_the_run_that_answers(pressing, now);
        }

        // Try to create a plan for goals if agent doesn't have one
        // Only do this periodically to avoid constant replanning
        if !agent.has_active_plan()
            && self.current_turn % crate::environment::seasons::ONCE_EVERY_FEW_DAYS == 0
        {
            // Use a default resource/return location (should be enhanced with real world data)
            let resource_loc = (50, 50, 0);
            let return_loc = (0, 0, 0);
            if agent.create_plan_for_goal(resource_loc, return_loc, self.current_turn) {
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
        if self.current_turn % crate::environment::seasons::ONCE_A_WEEK == 0 {
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

        // Check if agent should interact with storehouse (every 20 turns, or when Preparedness is high)
        // This happens independently of drive-based actions to enable cooperative resource sharing
        if self.current_turn % crate::environment::seasons::ONCE_EVERY_OTHER_DAY == 0 || {
            let agent = &self.population.agents[agent_index];
            agent.drives.get(DriveType::Preparedness)
                .map(|d| d.value > 0.6)
                .unwrap_or(false)
        } {
            // Only somebody standing at a storehouse looks in at one, and
            // what is counted is what is in that one (#357)
            let Some(store) = self.the_store_i_am_standing_at(agent_index) else {
                return;
            };
            let (storehouse_food, storehouse_resources) = {
                use crate::world::ItemType;
                let resource_types = [
                    ItemType::Wood, ItemType::Stone, ItemType::Iron,
                    ItemType::Clay, ItemType::Sand, ItemType::Coal,
                ];
                let held = self.world.pit_at(store).map(|pit| pit.holds.clone()).unwrap_or_default();
                let food_total: u32 = held
                    .iter()
                    .filter(|item| crate::world::nutrition::is_this_food(&item.item_id))
                    .map(|item| item.quantity)
                    .sum();
                let resource_total: u32 = held
                    .iter()
                    .filter(|item| {
                        crate::agents::storage_integration::id_to_item_type(&item.item_id)
                            .is_some_and(|kind| resource_types.contains(&kind))
                    })
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
