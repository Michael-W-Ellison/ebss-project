// src/analytics/wanting/errands.rs
//! Turning a want into a step somebody can actually take.
//!
//! Between deciding *what* and doing it there is a run of questions nobody
//! asks out loud: is the tool in the bag rather than the hand, are both hands
//! full, is the material even in this country, would two hours making a better
//! axe save six hours cutting - and, the one that makes a walk finishable, is
//! this errand worth turning round for.
//!
//! Part of the decision layer - see [`super`]. Nothing here does anything: it
//! answers what would be worth doing, and hands that answer back up the ladder.

use super::super::Simulation;
use crate::environment::Action;
use log::debug;

impl Simulation {
    /// Whether a garment of this warmth is worth making, given what is already
    /// on that slot
    pub(in crate::analytics) fn worth_making(warmth: f32, worn: f32) -> bool {
        warmth > worn * Self::WORTH_MAKING_ANEW + Self::WARMTH_WORTH_CHANGING_FOR
    }

    /// How far below its ideal an agent has to be to want another layer.
    ///
    /// Well short of `is_too_cold`, which is two degrees down and already
    /// dangerous: nobody waits until they are hypothermic to think about a
    /// coat, and an agent that did would spend the whole time it was cold
    /// walking to shelter instead of making one.
    pub(in crate::analytics) const CHILLY_MARGIN: f32 = 0.5;

    /// How far an agent will travel for the material to clothe itself.
    ///
    /// Further than it will go for food, because flax and cotton grow in a
    /// handful of patches on a map where there is something to eat almost
    /// everywhere - but not so far that the trip costs more than the coat is
    /// worth.
    pub(in crate::analytics) const CLOTHING_MATERIAL_RADIUS: u32 = 40;

    /// Insulation past which an agent counts itself dressed and gets on with
    /// its life.
    ///
    /// Without a stopping point this is a bottomless job. An unclothed agent
    /// sits about a degree under its ideal most of the time, so it is nearly
    /// always a little cold, and there is nearly always another patch of flax
    /// somewhere worth walking to: agents chased marginal warmth across the
    /// map instead of eating, and populations fell by a quarter.
    pub(in crate::analytics) const ENOUGH_INSULATION: f32 = 0.35;

    /// Whether the agent can spare the material for this garment.
    ///
    /// Wood is the one material that is wanted for something else: a fire
    /// takes ten and cooking is worth more than a pair of bark boots is, so
    /// wood only goes into clothing once there is a fire's worth left over.
    /// Without this agents made boots out of the firewood, stopped cooking,
    /// and went back to eating raw - four points of the fed population, for an
    /// insulation of about one part in a hundred.
    pub(in crate::analytics) fn can_spare_material(
        agent: &crate::agents::Agent,
        recipe: &crate::agents::equipment::GarmentRecipe,
    ) -> bool {
        let reserve = if recipe.material_item == "wood" {
            Self::FIRE_BUILD_WOOD + Self::FIRE_FUEL_WOOD
        } else {
            0
        };

        agent
            .inventory
            .has_item(recipe.material_item, recipe.material_amount + reserve)
    }

    /// The material for the warmest garment an agent could go and get, and the
    /// patch it grows in
    pub(in crate::analytics) fn material_to_gather(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> Option<(String, crate::world::Position)> {
        use crate::world::ResourceType;

        let quality = Self::expected_garment_quality(agent);

        crate::agents::equipment::GARMENT_RECIPES
            .iter()
            .filter(|recipe| {
                Self::worth_making(
                    Self::garment_warmth(recipe, quality),
                    Self::warmth_worn(agent, recipe.slot),
                )
            })
            .filter_map(|recipe| {
                let resource = match recipe.material_item {
                    "flax" => ResourceType::Flax,
                    "cotton" => ResourceType::Cotton,
                    "hides" => ResourceType::Hides,
                    "wool" => ResourceType::Wool,
                    "wood" => ResourceType::Wood,
                    _ => return None,
                };

                let patch = self.nearest_resource_within(
                    agent_position,
                    Self::CLOTHING_MATERIAL_RADIUS,
                    |node| node.resource_type == resource,
                )?;

                // Warmth is worth having, but not at any distance. A cloak's
                // worth of flax forty tiles off is a worse bargain than bark
                // from the trees an agent is standing in, and agents that
                // always went for the warmest thing walked instead of ate.
                let from = crate::world::Position::new(agent_position.0, agent_position.1);
                let travel = from.distance_to(&patch) as f32;
                let worth = Self::garment_warmth(recipe, quality) / (1.0 + travel / 10.0);

                Some((recipe.material_item.to_string(), patch, worth))
            })
            .max_by(|(_, _, a), (_, _, b)| {
                a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(material, patch, _)| (material, patch))
    }

    /// How close a hunter has to be to strike: a spear's throw, not a
    /// line of sight across the valley
    pub(in crate::analytics) const HUNT_REACH: i32 = 2;

    /// How close an animal has to be before a hungry agent turns aside for
    /// it.
    ///
    /// Short, and deliberately so. Crossing the valley after a deer is the
    /// expedition that does not pay and never did - measured, agents that
    /// went after every animal because their pack was empty starved for it
    /// and two settlements in forty died out. What this is for is the other
    /// case: something standing in front of you while the nearest berry is a
    /// walk away.
    pub(in crate::analytics) const AS_NEAR_AS_PREY_HAS_TO_BE_TO_BOTHER: i32 = 5;



    /// How far apart two people can be and still come to anything.
    pub(in crate::analytics) const CLOSE_ENOUGH_TO_COURT: i32 = 3;

    /// First step of a route from `from` to `target`, routing around obstacles.
    ///
    /// A breadth-first search over passable tiles, bounded so a walled-off
    /// destination cannot cost an unbounded scan. Stepping greedily toward the
    /// target instead traps agents against terrain: a lake between an agent
    /// and a berry patch leaves it stepping east, west, east forever, which is
    /// fatal when it is the trip to food that stalls.
    pub(in crate::analytics) fn next_step_toward(
        &self,
        from: (i32, i32, i32),
        target: (i32, i32, i32),
    ) -> Option<(i32, i32, i32)> {
        use std::collections::{BTreeMap, VecDeque};

        const MAX_VISITED: usize = 4096;

        let start = (from.0, from.1);
        let goal = (target.0, target.1);

        if start == goal {
            return None;
        }

        let mut queue = VecDeque::new();
        let mut came_from: BTreeMap<(i32, i32), (i32, i32)> = BTreeMap::new();

        queue.push_back(start);
        came_from.insert(start, start);

        let mut visited = 0usize;

        while let Some(current) = queue.pop_front() {
            visited += 1;
            if visited > MAX_VISITED {
                break;
            }

            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let next = (current.0 + dx, current.1 + dy);

                if came_from.contains_key(&next) {
                    continue;
                }

                // The goal tile itself may hold a building or resource the
                // agent is heading for, so only intermediate tiles must be
                // walkable.
                if next != goal && !self.is_passable_tile(next.0, next.1) {
                    continue;
                }

                came_from.insert(next, current);

                if next == goal {
                    let mut step = next;
                    while came_from[&step] != start {
                        step = came_from[&step];
                    }
                    return Some((step.0, step.1, from.2));
                }

                queue.push_back(next);
            }
        }

        None
    }

    /// Replace a move toward somewhere the agent cannot actually reach.
    ///
    /// A remembered patch can sit behind a lake or inside a pocket of terrain.
    /// Walking greedily at an unreachable target leaves the agent shuffling
    /// between two tiles forever, so the memory is dropped as unusable and the
    /// next-best survival option taken instead.
    /// Keep walking where this one was already walking.
    ///
    /// "Once an agent plans an action, it would not change its mind unless its
    /// situation changed in some manner... In most cases, the agents should
    /// not need to change their decisions once they are made."
    ///
    /// Every tile of every walk used to be a fresh decision, made from scratch
    /// against a world that had moved one step, so a twenty-tile trip to a
    /// fish run was twenty chances to be sent somewhere else. Measured, `Move`
    /// ran at a third of all turns and most of those trips did not finish;
    /// agents ate whatever was underfoot when they gave up, which is why
    /// choosing food by what it is *worth* measured worse than choosing the
    /// nearest thing.
    ///
    /// What ends an errand is a change in what the agent needs, and there are
    /// four of them. It has arrived. Something frightened it. A different
    /// drive has taken the lead - the drive demands changed, which is the one
    /// the specification names. Or the walk has run so far past what the
    /// distance was worth that the place is plainly not reachable, and going
    /// on is a way of starving politely.
    ///
    /// Note what is *not* on that list: a nearer patch coming into view, this
    /// turn's dice, or the same drive pressing slightly differently. Those are
    /// the things that used to turn an agent round.
    /// How much harder a drive has to press than the one an agent set out on
    /// before it turns them round, at the moment of setting out.
    ///
    /// A quarter again. Measured, a bare comparison abandoned 58% of errands
    /// mid-walk, because the two drives at the head of the queue trade places
    /// almost every turn.
    pub(in crate::analytics) const HOW_MUCH_HARDER_TO_TURN_SOMEBODY_ROUND: f32 = 1.25;

    /// And how much harder again by the time the errand is all but done.
    ///
    /// "If an agent is a few steps away from getting a meal and hydration
    /// drive suddenly kicks in, then the agent abandoning its current task to
    /// get a drink could waste the invested energy the agent spent to get a
    /// meal."
    ///
    /// The walk already made is spent whether the agent finishes or not, and
    /// turning round two paces from the patch throws all of it away to save
    /// two paces of the next trip. So what it takes to turn somebody round
    /// climbs with how much of the errand is behind them.
    pub(in crate::analytics) const WHAT_A_WALK_ALREADY_MADE_IS_WORTH: f32 = 1.5;

    /// How much harder a drive must press to turn this agent off this errand.
    ///
    /// Sunk cost, deliberately - which is a fallacy about *money already
    /// spent* and not about a walk half made. The half-made walk is not the
    /// sunk part: what is sunk is the energy, and what the nearness buys is
    /// the *rest* of the trip at a fraction of what a fresh one would cost.
    /// An agent two paces from a meal is two paces from a meal; one that turns
    /// round is twenty paces from the next one and has paid eighteen for
    /// nothing.
    ///
    /// It is a multiplier and not a veto, on purpose. `how_hard_it_presses`
    /// grows without bound as a killing drive nears its clock -
    /// `1.0 + deadly * SOONER_IS_WORSE` - so a body that will actually die of
    /// thirst still turns round, however near its supper is. What this stops
    /// is a drive merely crossing its threshold at an awkward moment.
    pub(crate) fn what_it_takes_to_turn_me_round(
        errand: &crate::agents::Errand,
        here: (i32, i32, i32),
    ) -> f32 {
        let still_to_go = errand.how_far_it_was(here) as f32;
        let already_walked = errand.turns_on_it as f32;
        let how_much_is_behind_me =
            already_walked / (already_walked + still_to_go).max(1.0);

        Self::HOW_MUCH_HARDER_TO_TURN_SOMEBODY_ROUND
            + how_much_is_behind_me * Self::WHAT_A_WALK_ALREADY_MADE_IS_WORTH
    }

    /// Put the errand down without throwing it away, and get on with whatever
    /// would not wait.
    ///
    /// An errand used to be destroyed the moment another need took the turn,
    /// and measured over six worlds that was **1,717 of the 3,047 a settlement
    /// set out on** - 56%. Of those, 1,401 were a primary need taking the turn
    /// from a secondary one: 1,062 of them a Preparedness errand cut short by
    /// hunger or thirst, which is to say every attempt at putting anything by,
    /// every time. A primary drive outranks a secondary one whatever its clock
    /// says, so this was not a rare interruption - it was the rule.
    ///
    /// A man who stops to drink has not changed his mind about the pit he was
    /// digging. He drinks, and goes back to it. What ends an errand is
    /// arriving, giving up on it, being frightened off it, or leaving it so
    /// long that the world has moved on - see `Errand::stale`.
    fn set_the_errand_aside(&mut self, agent_index: usize, action: Action) -> Action {
        let waited = {
            let Some(errand) = self.population.agents[agent_index].errand.as_mut() else {
                return action;
            };
            errand.set_aside += 1;
            errand.stale()
        };

        let why = if waited {
            self.population.agents[agent_index].errand = None;
            "errand: waited too long to be worth going back to"
        } else {
            "errand: set aside for something that would not wait"
        };

        *self.what_a_threat_came_to.entry(why.to_string()).or_insert(0) += 1;

        action
    }

    pub(in crate::analytics) fn stick_to_the_errand(
        &mut self,
        agent_index: usize,
        action: Action,
        running_away: bool,
    ) -> Action {
        let here = self.population.agents[agent_index].state.position;
        let presses_hardest = self.population.agents[agent_index].what_presses_hardest();

        // Something frightened it, or the threat tree took the turn. Whatever
        // it was going to do can wait.
        if running_away {
            self.population.agents[agent_index].errand = None;
            return action;
        }

        if let Some(errand) = self.population.agents[agent_index].errand.clone() {
            // A different drive at the head of the queue is not on its own a
            // change of situation: two drives within a whisker of each other
            // swap places every turn as one is nibbled at and the other
            // builds, and an agent that turns round each time they do gets
            // nowhere. What ends the errand is a drive that has actually taken
            // over - one pressing clearly harder than the one this one set out
            // to answer, as it presses *now*.
            let still_wants_it = presses_hardest == Some(errand.for_drive)
                || match presses_hardest {
                    Some(other) => {
                        let mine = self.population.agents[agent_index]
                            .how_hard_it_presses(errand.for_drive);
                        let theirs =
                            self.population.agents[agent_index].how_hard_it_presses(other);
                        theirs < mine * Self::what_it_takes_to_turn_me_round(&errand, here)
                    }
                    None => true,
                };
            let a_long_walk = errand
                .how_far_it_was(here)
                .max(crate::agents::Errand::AT_LEAST_THIS_MANY_TURNS)
                * crate::agents::Errand::HOW_LONG_A_WALK_IS_WORTH;
            let given_up = errand.turns_on_it > a_long_walk;

            // A making errand is finished when the thing is in the pack, and
            // is carried on with by taking the next step towards it. This is
            // the whole reason the tool ladder is climbable: a bow is four or
            // five turns of chain, and nobody ever took the second one.
            if let Some(wanted) = errand.to_make.clone() {
                let done = self.population.agents[agent_index].how_many_i_have(&wanted) > 0;

                // Something that will not wait has taken the turn. The making
                // is not abandoned for it - it is put down and picked up
                // again. See `Errand::set_aside`.
                if !done && !given_up && !still_wants_it {
                    return self.set_the_errand_aside(agent_index, action);
                }

                if done || given_up {
                    let why = if done {
                        "errand: made it"
                    } else {
                        "errand: gave up on the making"
                    };
                    *self.what_a_threat_came_to.entry(why.to_string()).or_insert(0) += 1;
                    self.population.agents[agent_index].errand = None;
                    return action;
                }

                match self.next_step_towards_making(&wanted, &self.population.agents[agent_index]) {
                    Some(step) => {
                        if let Some(errand) = self.population.agents[agent_index].errand.as_mut() {
                            errand.turns_on_it += 1;
                            errand.set_aside = 0;
                        }
                        *self
                            .what_a_threat_came_to
                            .entry("errand: another turn on the making".to_string())
                            .or_insert(0) += 1;
                        return step;
                    }
                    None => {
                        // The chain is short of something that has to be
                        // found, and finding it is not this errand
                        *self
                            .what_a_threat_came_to
                            .entry("errand: the making is stuck".to_string())
                            .or_insert(0) += 1;
                        self.population.agents[agent_index].errand = None;
                        return action;
                    }
                }
            }

            // The same, for a walk: going for a drink is not a change of mind.
            if !errand.arrived(here) && !given_up && !still_wants_it {
                return self.set_the_errand_aside(agent_index, action);
            }

            if errand.arrived(here) || given_up {
                let why = if errand.arrived(here) {
                    "errand: got there"
                } else {
                    "errand: gave up on it"
                };
                *self.what_a_threat_came_to.entry(why.to_string()).or_insert(0) += 1;
                self.population.agents[agent_index].errand = None;
                return action;
            }

            // Nothing has changed. Keep walking, and the waiting is over.
            if let Some(errand) = self.population.agents[agent_index].errand.as_mut() {
                errand.turns_on_it += 1;
                errand.set_aside = 0;
            }
            *self
                .what_a_threat_came_to
                .entry("errand: kept to it".to_string())
                .or_insert(0) += 1;
            return Action::Move {
                target: errand.going_to,
            };
        }

        // No errand. If this turn is the start of a walk, that is one.
        if let Action::Move { target } = action {
            if let Some(for_drive) = presses_hardest {
                let pressed_this_hard =
                    self.population.agents[agent_index].how_hard_it_presses(for_drive);
                self.population.agents[agent_index].errand = Some(crate::agents::Errand {
                    going_to: target,
                    to_make: None,
                    for_drive,
                    pressed_this_hard,
                    turns_on_it: 1,
                set_aside: 0,
                });
                *self
                    .what_a_threat_came_to
                    .entry("errand: set out".to_string())
                    .or_insert(0) += 1;
            }
        }

        action
    }

    pub(in crate::analytics) fn retarget_unreachable_move(&mut self, agent_index: usize, action: Action) -> Action {
        use crate::core::memory::SpatialMemoryType;

        let mut action = action;

        // Bounded: each pass drops one memory, so this cannot spin
        for _ in 0..4 {
            let target = match &action {
                Action::Move { target } => *target,
                _ => return action,
            };

            let position = self.population.agents[agent_index].state.position;

            if target == position || self.next_step_toward(position, target).is_some() {
                return action;
            }

            let forgotten = {
                let agent = &mut self.population.agents[agent_index];
                agent.memory.forget_location(SpatialMemoryType::Food, target)
            };

            if !forgotten {
                return action;
            }

            debug!(
                "Agent {} cannot reach remembered food at {:?}, forgetting it",
                self.population.agents[agent_index].id, target
            );

            let agent = &self.population.agents[agent_index];
            match self.survival_action(agent, position, false) {
                Some(next_action) => action = next_action,
                None => return action,
            }
        }

        action
    }

    /// Whether the hands doing this have what the verb wants in them.
    ///
    /// Returns what is missing, or `None` when the action can go ahead. The
    /// requirement comes from the matrix rather than from here: this function
    /// knows how to ask an agent what it is holding and nothing else about
    /// which verbs want what.
    /// Spend the turn getting the tool out, when the job about to be done
    /// wants one and it is still in the pack.
    ///
    /// The matrix already says which actions want a tool and for what trade,
    /// so this asks it rather than keeping a second list. It costs the agent
    /// a turn and buys back `WHAT_A_TOOL_STILL_IN_THE_PACK_IS_WORTH` on this
    /// piece of work and every piece after it until the thing is put away.
    pub(in crate::analytics) fn get_the_tool_out_for(&self, action: Action, agent_index: usize) -> Action {
        use crate::environment::verbs;

        // Freeing a hand is itself the answer to a different problem, and the
        // two must not fight over the turn
        if matches!(action, Action::Equip { .. } | Action::Unequip { .. }) {
            return action;
        }

        let agent = &self.population.agents[agent_index];

        if !agent.a_hand_to_spare() {
            return action;
        }

        let tried = crate::agents::Agent::what_was_tried(&action);
        let named = tried.split(':').next().unwrap_or(&tried);

        // A making can name a tool that has to be in the hand and is not used
        // up - a hammerstone is not part of the blade, it is what the blade is
        // beaten out with. The matrix does not know about those, because it is
        // keyed on the verb rather than on the recipe, so an agent who owned
        // the hammerstone and had not got it out was refused every time.
        if let Action::Craft { item_type } = &action {
            let wants = crate::environment::making::every_way_to_make(item_type)
                .filter(|step| agent.knows_how_to(step))
                .find_map(|step| step.wants_in_hand)
                .filter(|tool| agent.how_many_i_have(tool) > 0)
                .filter(|tool| !agent.is_in_my_hand(tool));

            if let Some(tool) = wants {
                return Action::Equip {
                    what: tool.to_string(),
                };
            }
        }

        let wanted = verbs::what_this_action_cannot_do_without(named);

        let out = wanted.iter().find_map(|wants| match wants {
            verbs::Wants::AToolFor(trade) => agent
                .what_i_have_to_work_with(*trade)
                .filter(|tool| !agent.is_in_my_hand(tool.called))
                .map(|tool| tool.called),
            _ => None,
        });

        match out {
            Some(what) => Action::Equip {
                what: what.to_string(),
            },
            None => action,
        }
    }

    /// Turn an action refused for want of a free hand into the act of freeing
    /// one.
    ///
    /// The matrix will refuse the job either way; the difference is whether
    /// the agent spends the turn failing or spends it putting the axe away.
    /// Anything else the matrix objects to - no tool at all, no vessel - is
    /// left alone, because emptying a hand does not help with those.
    /// What a step costs, as a multiple of what it costs empty-handed.
    ///
    /// Nothing at all up to `WHAT_GOES_UNNOTICED` of what a person can carry
    /// - a pack with a day's food and a spear in it is not a burden - and
    /// then rising to `WHAT_A_FULL_PACK_COSTS` at the limit of what the arms
    /// will hold.
    pub(in crate::analytics) fn what_this_load_costs(agent: &crate::agents::Agent) -> f32 {
        let capacity = agent.inventory.effective_max_weight();

        if capacity <= 0.0 {
            return 1.0;
        }

        let loaded = (agent.inventory.current_weight / capacity).clamp(0.0, 1.0);
        let felt = ((loaded - Self::WHAT_GOES_UNNOTICED) / (1.0 - Self::WHAT_GOES_UNNOTICED))
            .clamp(0.0, 1.0);

        1.0 + felt * (Self::WHAT_A_FULL_PACK_COSTS - 1.0)
    }

    /// The share of what somebody can carry that they carry without feeling
    /// it.
    pub(in crate::analytics) const WHAT_GOES_UNNOTICED: f32 = 0.4;

    /// What a step costs somebody loaded to the limit, against the same step
    /// taken empty-handed.
    pub(in crate::analytics) const WHAT_A_FULL_PACK_COSTS: f32 = 1.8;

    pub(in crate::analytics) fn free_a_hand_for(&self, action: Action, agent_index: usize) -> Action {
        use crate::environment::verbs;

        let agent = &self.population.agents[agent_index];

        if agent.a_hand_to_spare() {
            return action;
        }

        let tried = crate::agents::Agent::what_was_tried(&action);
        let named = tried.split(':').next().unwrap_or(&tried);

        let wants_a_hand = verbs::what_this_action_cannot_do_without(named)
            .iter()
            .any(|wants| matches!(wants, verbs::Wants::AFreeHand));

        if !wants_a_hand {
            return action;
        }

        match agent.what_i_would_put_away() {
            Some(what) => Action::Unequip { what },
            None => action,
        }
    }

    /// What the matrix says this action wants and these hands have not got.
    ///
    /// The structured answer. Two things ask it: the executor, to refuse, and
    /// the decision, to do something about it before the turn is spent - see
    /// `what_these_hands_are_short_of` and `make_what_this_wants`. Keeping one
    /// function between them is deliberate: two ways of asking whether a man
    /// can do a job is how this project has lost measurements before.
    pub(in crate::analytics) fn what_this_wants_that_is_missing(
        &self,
        action: &Action,
        agent_index: usize,
    ) -> Option<crate::environment::verbs::Wants> {
        self.what_this_one_is_short_of(action, &self.population.agents[agent_index])
    }

    /// The same question asked of an agent rather than of an index, so that
    /// the decision layer - which has no index - can ask it too.
    pub(in crate::analytics) fn what_this_one_is_short_of(
        &self,
        action: &Action,
        agent: &crate::agents::Agent,
    ) -> Option<crate::environment::verbs::Wants> {
        use crate::environment::verbs;

        // The action's bare name, which is how the matrix refers to it:
        // "gather:wood" is a gather
        let tried = crate::agents::Agent::what_was_tried(action);
        let named = tried.split(':').next().unwrap_or(&tried);

        let wanted = verbs::what_this_action_cannot_do_without(named);
        if wanted.is_empty() {
            return None;
        }

        let holding = |what: &str| agent.how_many_i_have(what);
        let helped_by = |trade| agent.what_i_have_to_work_with(trade).is_some();
        let a_hand_to_spare = agent.a_hand_to_spare();
        let carrying_liquid = agent.how_much_water_i_carry();

        wanted.into_iter().find(|wants| {
            !wants.satisfied_by_hands(&holding, &helped_by, a_hand_to_spare, carrying_liquid)
        })
    }

    /// The raw thing a tool's chain is waiting on, fetched now rather than
    /// whenever the Utility drive next wins an argument.
    ///
    /// `make_what_this_wants` stops at "no step can be taken", and past that
    /// point the chain is short of something that has to be found: a man who
    /// knows how to knap a knife and is standing in a meadow with no stone
    /// gets no further. Measured after that change, **1,690 short-handed
    /// refusals a world remained**, down from 2,695, and they are all this
    /// case.
    ///
    /// The machinery for it already existed and was in the wrong place.
    /// `Agent::what_i_must_find` sits at the *bottom* of the Utility chain,
    /// behind working, vessels, crafting, trading, stooping and taking from
    /// somebody - seven branches, on a drive that rarely wins against Hunger.
    /// It is the same defect `Craft` had, one link along, and it wants the
    /// same answer: fetching the stone is not what somebody does with a spare
    /// moment, it is what they do when they have found they need a knife.
    pub(in crate::analytics) fn fetch_what_the_making_of_it_wants(
        &self,
        action: Action,
        agent_index: usize,
        wanted: &str,
    ) -> Action {
        let agent = &self.population.agents[agent_index];

        let holding = |what: &str| agent.how_many_i_have(what);
        let knows = |step: &crate::environment::making::Making| agent.knows_how_to(step);

        let short_of =
            crate::environment::making::everything_wanting_knowing(wanted, &holding, &knows);

        // Something he has actually laid eyes on, and that is near enough for
        // the fetching to come to anything. Naming a thing this ground has not
        // got trades a refusal for want of a tool for a refusal for want of a
        // source, and this project has been round that loop before: a refusal
        // is worse than a wasted turn, because it goes into the record.
        let here = agent.state.position;
        let Some(raw) = short_of
            .iter()
            .filter(|what| agent.have_i_seen(what))
            .find(|what| self.could_this_gather_come_to_anything(agent, here, what))
        else {
            return action;
        };

        let instead = Action::Gather {
            resource_type: raw.to_string(),
        };

        if self
            .what_this_wants_that_is_missing(&instead, agent_index)
            .is_some()
        {
            return action;
        }

        instead
    }

    pub(in crate::analytics) fn what_these_hands_are_short_of(
        &self,
        action: &Action,
        agent_index: usize,
    ) -> Option<String> {
        use crate::environment::verbs;

        self.what_this_wants_that_is_missing(action, agent_index)
            .map(|wants| match wants {
                verbs::Wants::ThisInHand(what) => format!("No {what} in hand for that"),
                verbs::Wants::AToolFor(trade) => {
                    format!("Nothing in hand that is any use for {}", trade.name())
                }
                verbs::Wants::AFreeHand => "Both hands full".to_string(),
                verbs::Wants::AVessel => "Nothing to hold water in".to_string(),
                verbs::Wants::BareHands => "Nothing wanting".to_string(),
            })
    }

    /// A turn about to be spent on a refusal, spent on the tool instead.
    ///
    /// The same argument as `get_the_tool_out_for`, one step further back.
    /// That one says: reaching for a tool is not what somebody does with a
    /// spare moment, it is what they do just before using it. Neither is
    /// *making* one. Making sits in the Utility branch, behind two others,
    /// and Utility is a drive that rarely wins - so measured over eight
    /// worlds a settlement attempted `Work` 18,756 times and was refused
    /// **88.2%** of them for want of a tool, `Excavate` 6,348 times and was
    /// refused **99.4%**, while every man alive knew how to make a handaxe
    /// and 2.8% of them owned one. Twenty-two thousand turns went on wanting
    /// a thing nobody would spend a turn making.
    ///
    /// So when the matrix is about to refuse an action for want of a tool,
    /// and this one knows a step towards that tool it could take right now,
    /// it takes the step. The turn was lost either way.
    /// Which trade a job wants, whether or not it has a tool to want.
    ///
    /// `what_this_wants_that_is_missing` only names a trade when the action
    /// cannot be done at all without a tool. This is the other question - what
    /// hand is this work done with - which is what has to be asked before
    /// anybody can weigh a better tool against the one they have.
    pub(in crate::analytics) fn what_trade_this_asks_for(
        &self,
        action: &Action,
        agent_index: usize,
    ) -> Option<crate::agents::skills::SkillType> {
        use crate::agents::skills::SkillType;

        Some(match action {
            // The gather asks for whatever the thing is gathered with. Named
            // by the request rather than by the node, because the request is
            // what the agent has decided to do.
            Action::Gather { resource_type } => {
                let _ = agent_index;
                match resource_type.as_str() {
                    "wood" => SkillType::Woodcutting,
                    "stone" | "iron" | "coal" | "clay" | "sand" => SkillType::Mining,
                    "grain" | "flax" | "cotton" => SkillType::Farming,
                    "food" | "greens" | "roots" | "herbs" => SkillType::Herbalism,
                    "fish" => SkillType::Fishing,
                    // Water wants nothing but hands, and a request for
                    // something with no trade behind it is not a job a tool
                    // makes faster
                    _ => return None,
                }
            }
            Action::Fish { .. } => SkillType::Fishing,
            Action::Hunt { .. } => SkillType::Hunting,
            Action::Build { .. } => SkillType::Construction,
            _ => return None,
        })
    }

    /// Stop and make a better tool, when the tool will save more than it costs.
    ///
    /// "The agent should look at the drive, their skills, the availability of
    /// tools to decrease time, if they need to make any tools, and decide the
    /// quickest method of satisfying their most important drive." And, from
    /// the efficiency specification before it: "eight hours with this axe, or
    /// two hours making a better one and six with that."
    ///
    /// `make_what_this_wants` already covers the case where the job is
    /// impossible without a tool. This is the case the model has never had:
    /// the job is perfectly possible, and doing it badly for the rest of the
    /// season is the more expensive of the two. `Tool::how_much_better` has
    /// been in the data since the tools were written and multiplied what came
    /// *off* a job and nothing else, so a stone axe and a bronze axe felled a
    /// tree at the same price and nobody ever had a reason to upgrade.
    ///
    /// The arithmetic is the specification's, and every term in it is a figure
    /// this model already keeps:
    ///
    /// - `Tool::how_long_it_lasts` is how many pieces of work the new tool has
    ///   in it. That is the horizon, and it is the honest one: a tool has to
    ///   pay for itself inside its own working life, and nothing has to be
    ///   assumed about how long the agent will go on wanting the trade.
    /// - `how_much_my_tools_help` is what the work costs now, and
    ///   `how_much_better` what it would cost after.
    /// - `how_many_turns_to_make` is the price, counted along the same chain
    ///   the agent will actually walk.
    ///
    /// So the saving is the work the tool has in it, at the difference between
    /// the two rates, and it is worth stopping when that beats the making.
    pub(in crate::analytics) fn would_a_better_tool_pay(&mut self, action: Action, agent_index: usize) -> Action {
        // Making something is not a job to interrupt with more making, and a
        // survival action is not one to interrupt at all: a starving man does
        // not knap a better knife first.
        if matches!(
            action,
            Action::Craft { .. } | Action::Equip { .. } | Action::Unequip { .. } | Action::Eat { .. }
        ) {
            return action;
        }

        let Some(trade) = self.what_trade_this_asks_for(&action, agent_index) else {
            return action;
        };

        let agent = &self.population.agents[agent_index];

        // A body that is actually in trouble works with what it has.
        if agent.state.physiology.is_starving() {
            return action;
        }

        // And so does a settlement that has nothing in for tonight.
        //
        // "Once basic survival needs can be satisfied over the long term,
        // other concerns start coming into play." A better axe is one of the
        // other concerns: it pays back over forty jobs, and forty jobs is no
        // use to somebody who will not see the week. `is_starving` alone is
        // too late a test - it wants three days into the reserve, and a people
        // permanently a third short of food is hungry long before that and
        // never technically starving. The larder's own bottom rung is the
        // right question and it is already being asked every turn.
        //
        // Measured without this: 1630, 1494 and 1387 mean last-alive against
        // 1612, 1552 and 1633 before the ladder, because a hundred and ninety
        // turns a run went on tools in settlements that needed the turns for
        // supper.
        if agent
            .state
            .what_the_larder_says
            .as_ref()
            .is_some_and(|larder| larder.rung == crate::agents::provision::HowLongTheFoodLasts::NotTheDay)
        {
            *self
                .what_a_threat_came_to
                .entry("tool: nothing in for tonight, so no".to_string())
                .or_insert(0) += 1;
            return action;
        }

        let Some(better) = agent.what_i_would_rather_have(trade) else {
            return action;
        };

        let now = agent.how_much_my_tools_help(trade).max(0.01);
        let after = better.how_much_better.max(now);
        if after <= now {
            return action;
        }

        let holding = |what: &str| agent.how_many_i_have(what);
        let knows = |step: &crate::environment::making::Making| agent.knows_how_to(step);
        let Some(costs) =
            crate::environment::making::how_many_turns_to_make(better.called, &holding, &knows)
        else {
            return action;
        };
        if costs == 0 {
            return action;
        }

        // What the tool has in it, at the difference the tool makes
        let saves = better.how_long_it_lasts * (1.0 / now - 1.0 / after);

        if saves <= costs as f32 {
            *self
                .what_a_threat_came_to
                .entry("tool: not worth the making".to_string())
                .or_insert(0) += 1;
            return action;
        }

        // It pays. Take it on as an errand, so that the chain is finished
        // rather than started over and over - see `Errand::to_make`.
        //
        // Take the turn on the next step towards it.
        //
        // Walked here rather than handed to `make_what_this_wants`, which
        // refuses a `Craft` on sight - it exists to rescue a job that *cannot*
        // be done, and a craft that wants a craft is a loop. Asking it for a
        // step towards a tool got the action straight back and counted as "no
        // step available" a hundred and sixteen times in a run while never
        // once diverting a turn.
        let in_hand = |what: &str| agent.how_many_i_have(what) > 0;
        let a_fire_is_to_hand = self
            .nearest_fire_from(agent.state.position, Self::FIRE_REACH, true)
            .is_some();
        let step = crate::environment::making::what_to_do_first_that_can_be_done(
            better.called,
            &holding,
            &knows,
            &in_hand,
            a_fire_is_to_hand,
        );

        let Some(step) = step else {
            // Short of something that has to be *found* rather than made, so
            // going and getting it is the job in hand. This was a dead end and
            // it was the commonest one by a distance - a hundred and three
            // turns in a run where the arithmetic said the tool was worth
            // having and the agent stood there for want of a length of flax.
            // `fetch_what_the_making_of_it_wants` is the same errand
            // `make_what_this_wants` runs one case over.
            *self
                .what_a_threat_came_to
                .entry("tool: gone to fetch what the making wants".to_string())
                .or_insert(0) += 1;
            return self.fetch_what_the_making_of_it_wants(action, agent_index, better.called);
        };

        let instead = Action::Craft {
            item_type: step.makes.to_string(),
        };

        // And the step must not be short-handed itself, or this trades a job
        // that works for a refusal and calls it planning
        if self
            .what_this_wants_that_is_missing(&instead, agent_index)
            .is_some()
        {
            *self
                .what_a_threat_came_to
                .entry("tool: worth making, but short-handed for the step".to_string())
                .or_insert(0) += 1;
            return action;
        }

        *self
            .what_a_threat_came_to
            .entry("tool: stopped to make a better one".to_string())
            .or_insert(0) += 1;

        if let Some(for_drive) = self.population.agents[agent_index].what_presses_hardest() {
            let pressed_this_hard =
                self.population.agents[agent_index].how_hard_it_presses(for_drive);
            let here = self.population.agents[agent_index].state.position;
            self.population.agents[agent_index].errand = Some(crate::agents::Errand {
                going_to: here,
                to_make: Some(better.called.to_string()),
                for_drive,
                pressed_this_hard,
                turns_on_it: 1,
                set_aside: 0,
            });
        }

        instead
    }

    /// The next thing to do towards making a named thing, if anything can be.
    ///
    /// The same walk `would_a_better_tool_pay` makes, pulled out so that an
    /// errand can go on making it turn after turn.
    pub(in crate::analytics) fn next_step_towards_making(
        &self,
        wanted: &str,
        agent: &crate::agents::Agent,
    ) -> Option<Action> {
        let holding = |what: &str| agent.how_many_i_have(what);
        let knows = |step: &crate::environment::making::Making| agent.knows_how_to(step);
        let in_hand = |what: &str| agent.how_many_i_have(what) > 0;
        let a_fire_is_to_hand = self
            .nearest_fire_from(agent.state.position, Self::FIRE_REACH, true)
            .is_some();

        let step = crate::environment::making::what_to_do_first_that_can_be_done(
            wanted,
            &holding,
            &knows,
            &in_hand,
            a_fire_is_to_hand,
        )?;

        let instead = Action::Craft {
            item_type: step.makes.to_string(),
        };
        if self.what_this_one_is_short_of(&instead, agent).is_some() {
            return None;
        }
        Some(instead)
    }

    /// The next thing to do towards having one of these in the pack: the
    /// making step if one can be taken, and otherwise going and getting
    /// whatever the chain is short of.
    ///
    /// The two halves already existed and were written out three times over -
    /// in `next_step_towards_making`, inside `would_a_better_tool_pay`, and
    /// again in `make_what_this_wants` - each of which then fell back to
    /// `fetch_what_the_making_of_it_wants` in its own way. This is the pair as
    /// one question, so that anybody who wants a thing can ask it.
    ///
    /// `None` means there is no step from here: the chain wants something this
    /// one has never seen, or that this ground has not got.
    pub(in crate::analytics) fn how_i_would_come_by(
        &self,
        wanted: &str,
        agent: &crate::agents::Agent,
    ) -> Option<Action> {
        if let Some(step) = self.next_step_towards_making(wanted, agent) {
            return Some(step);
        }

        let holding = |what: &str| agent.how_many_i_have(what);
        let knows = |step: &crate::environment::making::Making| agent.knows_how_to(step);
        let short_of =
            crate::environment::making::everything_wanting_knowing(wanted, &holding, &knows);

        let here = agent.state.position;
        let raw = short_of
            .iter()
            .filter(|what| agent.have_i_seen(what))
            .find(|what| self.could_this_gather_come_to_anything(agent, here, what))?;

        let instead = Action::Gather {
            resource_type: raw.to_string(),
        };

        if self.what_this_one_is_short_of(&instead, agent).is_some() {
            return None;
        }

        Some(instead)
    }

    pub(in crate::analytics) fn make_what_this_wants(&mut self, action: Action, agent_index: usize) -> Action {
        use crate::environment::verbs::Wants;

        // Making a thing is itself an answer to this, and the two must not
        // fight over the turn. `Work` is deliberately *not* on this list: a
        // working refused for want of a knife is exactly the case, and the
        // guard against a making that wants a tool of its own is below, where
        // the substitute is checked before it is taken.
        if matches!(
            action,
            Action::Craft { .. } | Action::Equip { .. } | Action::Unequip { .. }
        ) {
            return action;
        }

        let Some(missing) = self.what_this_wants_that_is_missing(&action, agent_index) else {
            return action;
        };

        let agent = &self.population.agents[agent_index];

        // What would answer it. A free hand is somebody else's problem - see
        // `free_a_hand_for` - and bare hands are never missing.
        let wanted: &str = match missing {
            Wants::ThisInHand(what) => what,
            Wants::AToolFor(trade) => match agent.what_i_would_rather_have(trade) {
                Some(tool) => tool.called,
                None => return action,
            },
            Wants::AVessel | Wants::AFreeHand | Wants::BareHands => return action,
        };

        // Only a step that can actually be carried out, and failing that the
        // raw thing the chain is short of. Naming a step that cannot be taken
        // is worse than the refusal it replaces: the refusal goes into the
        // record and the man learns from it that making knives does not work.
        // See `how_i_would_come_by`.
        let wanted = wanted.to_string();
        let Some(instead) = self.how_i_would_come_by(&wanted, agent) else {
            return action;
        };

        // And take it on as an errand, so the chain is finished rather than
        // started over and over.
        //
        // `would_a_better_tool_pay` has done this since `Errand::to_make` was
        // written and this one never did, which is the same defect one link
        // along: a turn was diverted onto the first step of a making and the
        // turn after that the whole decision was made again from scratch.
        // Measured over six worlds, twenty turns went on the first step of a
        // hunting tool and not one of them was ever followed up.
        self.take_the_making_on(&wanted, agent_index);

        instead
    }

    /// Take a making on as an errand, so that a chain several turns long is
    /// walked rather than restarted.
    fn take_the_making_on(&mut self, wanted: &str, agent_index: usize) {
        let Some(for_drive) = self.population.agents[agent_index].what_presses_hardest() else {
            return;
        };

        let pressed_this_hard =
            self.population.agents[agent_index].how_hard_it_presses(for_drive);
        let here = self.population.agents[agent_index].state.position;

        self.population.agents[agent_index].errand = Some(crate::agents::Errand {
            going_to: here,
            to_make: Some(wanted.to_string()),
            for_drive,
            pressed_this_hard,
            turns_on_it: 1,
                set_aside: 0,
        });
    }
}
