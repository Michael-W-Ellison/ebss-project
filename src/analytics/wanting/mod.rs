// src/analytics/wanting/mod.rs
//! Wanting: given a drive, what would answer it?
//!
//! This is the layer the whole model turns on, and until now it was scattered
//! through a sixteen-thousand-line file with no boundary at all - the ladder in
//! one place, what hunger asks for eight hundred lines further down, what the
//! errand machinery does with the answer three thousand lines after that.
//!
//! The ladder is here in this file. Everything below it is one question per
//! module, named for what it is about rather than for the drive that happens to
//! ask it, because more than one drive asks most of them:
//!
//! - [`food`] - hunger and thirst, the two that kill
//! - [`quarry`] - hunting and fishing
//! - [`ground`] - working the soil before there is anything on it to take
//! - [`store`] - putting by, and taking out again
//! - [`shelter`] - keeping warm and dry
//! - [`camp`] - whether to stay, and where to go instead
//! - [`errands`] - turning a want into a step somebody can actually take
//!
//! **Nothing in here does anything.** Every function answers a question and
//! hands the answer back; the doing is in [`crate::analytics::doing`] and the
//! order of a turn is in [`crate::analytics::turn`]. That separation is the
//! point of the boundary: a change to what hunger asks for cannot now quietly
//! change what eating does.
//!
//! The move was behaviour-neutral, and proved so: three seeds run six hundred
//! ticks give byte-identical worlds either side of it.

pub mod camp;
pub mod errands;
pub mod food;
pub mod ground;
pub mod quarry;
pub mod shelter;
pub mod store;

use super::Simulation;
use crate::agents::practices::Circumstance;
use crate::core::DriveType;
use crate::environment::Action;

impl Simulation {
    /// Generate an action based on recent percepts (if high-salience percepts exist)
    /// Returns None if no percept warrants immediate action
    pub(in crate::analytics) fn generate_action_from_percepts(
        recent_percepts: &[(u32, crate::agents::sensory_processing::Percept)],
        agent_drives: &crate::core::DriveState,
        agent_position: (i32, i32, i32),
    ) -> Option<Action> {
        use crate::agents::sensory_processing::{Percept, calculate_salience};

        if recent_percepts.is_empty() {
            return None;
        }

        // Find the most salient recent percept
        let most_salient = recent_percepts.iter()
            .max_by(|(_, a), (_, b)| {
                let sal_a = calculate_salience(a, agent_drives);
                let sal_b = calculate_salience(b, agent_drives);
                sal_a.partial_cmp(&sal_b).unwrap_or(std::cmp::Ordering::Equal)
            });

        if let Some((_, percept)) = most_salient {
            let salience = calculate_salience(percept, agent_drives);

            // Only override drive-based actions if salience is high (> 0.7)
            if salience > 0.7 {
                match percept {
                    Percept::DangerDetected { threat_type: _, position, severity } => {
                        // High-priority: flee from danger
                        if let Some(danger_pos) = position {
                            // Move away from danger position
                            let dx = agent_position.0 - danger_pos.0;
                            let dy = agent_position.1 - danger_pos.1;

                            // Normalize and extend to flee further
                            let distance = ((dx * dx + dy * dy) as f32).sqrt().max(1.0);
                            let flee_distance = (severity * 15.0) as i32;

                            let flee_x = agent_position.0 + ((dx as f32 / distance) * flee_distance as f32) as i32;
                            let flee_y = agent_position.1 + ((dy as f32 / distance) * flee_distance as f32) as i32;

                            return Some(Action::Move {
                                target: (flee_x, flee_y, agent_position.2),
                            });
                        } else {
                            // Unknown danger location - move to random safe spot
                            use rand::Rng;
                            let mut rng = crate::core::dice::roll();
                            let safe_x = agent_position.0 + rng.gen_range(-10..=10);
                            let safe_y = agent_position.1 + rng.gen_range(-10..=10);

                            return Some(Action::Move {
                                target: (safe_x, safe_y, agent_position.2),
                            });
                        }
                    }
                    Percept::ResourceDetected {  position, .. } => {
                        // High-salience resource (usually means high hunger/thirst)
                        // Move towards it
                        return Some(Action::Move {
                            target: *position,
                        });
                    }
                    Percept::AgentDetected { agent_id,  .. } => {
                        // High-salience agent (usually means high social drive)
                        // Attempt social interaction
                        return Some(Action::Socialize {
                            target_agent_id: *agent_id,
                        });
                    }
                    _ => {
                        // Other percepts don't warrant action override
                        return None;
                    }
                }
            }
        }

        None
    }

    /// Find the nearest agent to use as a social interaction target
    /// Returns None if no suitable target is found
    pub(in crate::analytics) fn find_nearest_social_target(
        agent_id: uuid::Uuid,
        position: (i32, i32, i32),
        agents: &[(uuid::Uuid, (i32, i32, i32))],
    ) -> Option<uuid::Uuid> {
        agents
            .iter()
            .filter(|(id, _)| *id != agent_id) // Exclude self
            // Near enough to say it to. Without this the nearest person *on
            // the map* was the answer, however far off that was - see
            // `Simulation::WITHIN_TALKING_DISTANCE`.
            .filter(|(_, pos)| Self::near_enough_to_talk(position, *pos))
            .min_by_key(|(_, pos)| {
                let dx = (pos.0 - position.0).abs();
                let dy = (pos.1 - position.1).abs();
                dx + dy // Manhattan distance
            })
            .map(|(id, _)| *id)
    }

    /// Resolve a nil UUID in an action to an actual nearby agent
    pub(in crate::analytics) fn resolve_action_target(
        action: Action,
        agent_id: uuid::Uuid,
        position: (i32, i32, i32),
        nearby_agents: &[(uuid::Uuid, (i32, i32, i32))],
    ) -> Action {
        match action {
            Action::Socialize { target_agent_id } if target_agent_id.is_nil() => {
                if let Some(target) = Self::find_nearest_social_target(agent_id, position, nearby_agents) {
                    Action::Socialize { target_agent_id: target }
                } else {
                    // No nearby agents, fall back to waiting
                    Action::Wait
                }
            }
            Action::ShareInformation { target_agent_id } if target_agent_id.is_nil() => {
                if let Some(target) = Self::find_nearest_social_target(agent_id, position, nearby_agents) {
                    Action::ShareInformation { target_agent_id: target }
                } else {
                    Action::Wait
                }
            }
            Action::Mate { target_agent_id } if target_agent_id.is_nil() => {
                if let Some(target) = Self::find_nearest_social_target(agent_id, position, nearby_agents) {
                    Action::Mate { target_agent_id: target }
                } else {
                    Action::Wait
                }
            }
            other => other, // Return unchanged
        }
    }

    /// Generate an action based on drive type and position
    pub(in crate::analytics) fn generate_action_for_drive(drive_type: DriveType, position: (i32, i32, i32)) -> Action {
        use rand::Rng;
        let mut rng = crate::core::dice::roll();

        // Map drive type to a representative action
        match drive_type {
            DriveType::Hunger => Action::Eat { food_type: "generic".to_string() },
            DriveType::Thirst => Action::Gather { resource_type: "water".to_string() },
            DriveType::Rest => Action::Sleep { duration: 10 },
            DriveType::Shelter => Action::Gather { resource_type: "hides".to_string() },
            // Building with nothing to build from is the commonest wasted
            // turn in the model - the drive path above checks the pack first.
            // A trip out for timber is what this falls back to.
            DriveType::Construction => Action::Gather { resource_type: "wood".to_string() },
            DriveType::Industry => Action::Gather { resource_type: "generic".to_string() },
            // Answered by going to the children, which needs to know where
            // they are - see `protective_action`. On its own it comes to
            // waiting where they last were.
            DriveType::Protection => Action::Wait,
            DriveType::Curiosity => {
                // Explore by moving to a random distant location
                let target_x = position.0 + rng.gen_range(-20..=20);
                let target_y = position.1 + rng.gen_range(-20..=20);
                Action::Move { target: (target_x, target_y, position.2) }
            },
            DriveType::Social => {
                // 50% chance to share information, 50% to socialize
                if rng.gen_bool(0.5) {
                    Action::ShareInformation { target_agent_id: uuid::Uuid::nil() }
                } else {
                    Action::Socialize { target_agent_id: uuid::Uuid::nil() }
                }
            },
            DriveType::Utility => Action::Craft { item_type: "spear".to_string() },
            // Putting something by needs something to put by, which this
            // ladder cannot see - the drive path above names a real thing out
            // of the agent's own pack. A trip out is the honest fallback.
            DriveType::Preparedness => Action::Gather { resource_type: "wood".to_string() },
            DriveType::Sustenance => Action::Gather { resource_type: "food".to_string() },
            DriveType::Safety => {
                // Move to a random nearby safe location
                let target_x = position.0 + rng.gen_range(-5..=5);
                let target_y = position.1 + rng.gen_range(-5..=5);
                Action::Move { target: (target_x, target_y, position.2) }
            },
            // Proposing to whoever is nearest is how Mate came to be a fifth
            // of everything a settlement did and to fail 99.9% of the time.
            // The drive path above finds somebody who could actually have a
            // child and whom this agent trusts.
            DriveType::Reproduction => Action::Wait,
            DriveType::Luxury => Action::Gather { resource_type: "luxury".to_string() },
        }
    }

    /// Generate an action based on an active goal
    pub(in crate::analytics) fn generate_action_for_goal(
        goal: &crate::core::goals::Goal,
        position: (i32, i32, i32),
        _fallback_drive: DriveType,
    ) -> Option<Action> {
        use crate::core::goals::{InternalGoal, ExternalGoal};
        use crate::core::EmotionType;

        // Check if it's an internal goal
        if let Some(internal) = &goal.internal {
            match internal {
                InternalGoal::IncreaseEmotion(emotion_type, _target) => {
                    // Map emotions to actions that satisfy them
                    match emotion_type {
                        EmotionType::Happiness => Some(Action::Socialize { target_agent_id: uuid::Uuid::nil() }),
                        EmotionType::Curiosity => Some(Action::Move {
                            target: (position.0 + 10, position.1 + 10, position.2)
                        }),
                        _ => None,
                    }
                },
                InternalGoal::DecreaseEmotion(emotion_type, _target) => {
                    match emotion_type {
                        EmotionType::Fear => Some(Action::SeekShelter),
                        EmotionType::Anger => Some(Action::Sleep { duration: 10 }),
                        EmotionType::Sadness => Some(Action::Socialize { target_agent_id: uuid::Uuid::nil() }),
                        _ => None,
                    }
                },
                InternalGoal::MaintainWellBeing(_threshold) => {
                    Some(Action::Sleep { duration: 10 })
                },
                InternalGoal::ReduceStress => {
                    Some(Action::Sleep { duration: 10 })
                },
                InternalGoal::SeekEntertainment => {
                    Some(Action::Socialize { target_agent_id: uuid::Uuid::nil() })
                },
            }
        // Check if it's an external goal
        } else if let Some(external) = &goal.external {
            match external {
                ExternalGoal::OwnHouse => {
                    Some(Action::Build {
                        structure_type: "house".to_string(),
                        position,
                    })
                },
                ExternalGoal::StockHouseFood(_amount) => {
                    Some(Action::Gather { resource_type: "food".to_string() })
                },
                ExternalGoal::ContributeFoodToStorehouse(amount) => {
                    Some(Action::Store {
                        item_type: "food".to_string(),
                        amount: *amount,
                    })
                },
                ExternalGoal::ObtainProtection => {
                    Some(Action::Craft { item_type: "leatherarmor".to_string() })
                },
                ExternalGoal::CraftItem(item_name) => {
                    Some(Action::Craft { item_type: item_name.clone() })
                },
                ExternalGoal::BuildStructure(structure_name) => {
                    Some(Action::Build {
                        structure_type: structure_name.clone(),
                        position,
                    })
                },
                ExternalGoal::GatherResource(resource_name, _amount) => {
                    Some(Action::Gather { resource_type: resource_name.clone() })
                },
                ExternalGoal::LearnSkill(skill_name) => {
                    // Learning happens through practice - map skill to relevant action
                    let skill_lower = skill_name.to_lowercase();
                    if skill_lower.contains("mining") {
                        Some(Action::Gather { resource_type: "stone".to_string() })
                    } else if skill_lower.contains("woodcutting") || skill_lower.contains("carpentry") {
                        Some(Action::Gather { resource_type: "wood".to_string() })
                    } else if skill_lower.contains("crafting") || skill_lower.contains("metalworking") {
                        Some(Action::Craft { item_type: "tool".to_string() })
                    } else if skill_lower.contains("construction") || skill_lower.contains("masonry") {
                        Some(Action::Build { structure_type: "structure".to_string(), position })
                    } else if skill_lower.contains("farming") || skill_lower.contains("herbalism") {
                        Some(Action::Gather { resource_type: "food".to_string() })
                    } else if skill_lower.contains("cooking") || skill_lower.contains("smelting") {
                        Some(Action::Craft { item_type: "processed".to_string() })
                    } else if skill_lower.contains("hunting") || skill_lower.contains("combat") || skill_lower.contains("archery") {
                        Some(Action::Hunt { animal_id: uuid::Uuid::nil(), weapon: None })
                    } else if skill_lower.contains("fishing") {
                        Some(Action::Gather { resource_type: "fish".to_string() })
                    } else if skill_lower.contains("social") {
                        Some(Action::Socialize { target_agent_id: uuid::Uuid::nil() })
                    } else if skill_lower.contains("navigation") {
                        Some(Action::Explore { direction: (1, 0, 0) })
                    } else {
                        // Default: generic crafting to practice skills
                        Some(Action::Craft { item_type: "practice".to_string() })
                    }
                },
                ExternalGoal::FormRelationship(_relationship_type) => {
                    Some(Action::Socialize { target_agent_id: uuid::Uuid::nil() })
                },
                ExternalGoal::CompleteJob(_job_name) => {
                    // Jobs are complex, fall back to drive-based action
                    None
                },
                ExternalGoal::ContributeMaterialsToStorehouse(amount) => {
                    Some(Action::Store {
                        item_type: "resource".to_string(),
                        amount: *amount,
                    })
                },
                ExternalGoal::EnsureToolsAvailable(_count) => {
                    Some(Action::Craft { item_type: "spear".to_string() })
                },
            }
        } else {
            // Goal has neither internal nor external set (shouldn't happen)
            None
        }
    }

    /// What an agent does when nothing has frightened it, in order:
    ///
    /// 1. stay alive - eat, drink, sleep
    /// 2. put on or make clothing, if it can be done where it stands
    /// 3. get out of the weather
    /// 4. cook what it is carrying
    /// 5. go after an animal, for the meat or the skin
    /// 6. go and get the material to clothe itself
    /// 7. act on something it can see or smell
    /// 8. carry on with a plan
    /// 9. work towards a goal
    /// 10. whatever its most pressing drive suggests
    /// What an agent does, when nothing has frightened or angered it.
    ///
    /// This used to be thirteen fixed priorities with the drives consulted at
    /// the thirteenth, which meant the drives decided almost nothing: seventy-
    /// nine per cent of everything a settlement did was foraging chosen off
    /// the ladder before a drive was ever asked, and `Action::Build` and
    /// `Action::Socialize` were chosen zero times in seven hundred and
    /// seventy-seven agent-lives. Giving every agent a personality changed
    /// nothing measurable for the same reason: personality reaches the drives,
    /// and the drives reached nothing.
    ///
    /// It is the other way round now. Two things pre-empt, because they are
    /// emergencies rather than wants; after that the needs are ranked by how
    /// hard each is pressing - see `Agent::how_hard_it_presses` - and the
    /// first that can actually be answered here and now takes the turn.
    pub(in crate::analytics) fn generate_non_emotional_action(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> (Action, bool) {
        // A child of one's own in trouble. Not a want: a parent goes, and the
        // Protection *drive* underneath is a tertiary disposition that waits
        // its turn like anything else.
        if let Some(action) = self.protective_action(agent, agent_position) {
            return (action, false);
        }

        // And freezing, where there is a roof within reach. Exposure is
        // already doing damage by the time this fires, so it is not a matter
        // of how much the agent wants to be warm.
        if agent.needs_shelter() && self.nearest_shelter_from(agent_position).is_some() {
            return (Action::SeekShelter, false);
        }

        // Everything else the agent wants, in the order it wants it
        let mut ranked: Vec<(DriveType, f32)> = DriveType::all()
            .into_iter()
            .map(|drive_type| (drive_type, agent.how_hard_it_presses(drive_type)))
            .filter(|(_, pressing)| *pressing > 0.0)
            .collect();

        ranked.sort_by(|a, b| {
            b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal)
        });

        // What the world is doing, once, for the whole of this turn's
        // deliberation: every drive's answer is judged against the same
        // afternoon.
        let here = self.what_it_is_like_here(agent, agent_position);

        for (drive_type, _) in ranked {
            if let Some(action) =
                self.how_this_agent_answers(drive_type, agent, agent_position, &here)
            {
                return (action, false);
            }
        }

        // Nothing is pressing and nothing is wanted, which is when an agent
        // gets to follow its own nose: what it has just noticed, then what it
        // had planned, then what it was working towards.
        let recent_percepts: Vec<(u32, crate::agents::sensory_processing::Percept)> =
            agent.recent_percepts.iter().cloned().collect();

        if let Some(percept_action) = Self::generate_action_from_percepts(
            &recent_percepts,
            &agent.drives,
            agent_position,
        ) {
            return (percept_action, false);
        }

        if agent.should_execute_plan() {
            if let Some(plan_action) = agent.get_plan_action() {
                return (plan_action, true);
            }
        }

        if let Some(goal) = agent.goals.highest_priority_goal() {
            let fallback_drive = agent
                .what_presses_hardest()
                .unwrap_or(DriveType::Curiosity);

            if let Some(goal_action) =
                Self::generate_action_for_goal(&goal, agent_position, fallback_drive)
            {
                return (goal_action, false);
            }
        }

        (Action::Wait, false)
    }

    /// How this agent would go about answering that need, if it can at all.
    ///
    /// `None` means this particular need has no answer available here and now -
    /// a wish to build with nowhere to build, a wish for company with nobody
    /// about - and the turn passes to whatever is pressing next. That is the
    /// part the old ladder could not do: it had one fixed order for everybody
    /// and no way for a need to stand aside.
    pub(in crate::analytics) fn how_this_agent_answers(
        &self,
        drive_type: DriveType,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
        here: &[Circumstance],
    ) -> Option<Action> {
        // "When an action fails to satisfy a drive, its odds of repeating
        // should decrease. Inversely, when an action satisfies a drive, its
        // odds of repeating should increase."
        //
        // Every attempt has been recorded against the particular thing tried
        // since `Lessons` was written, and nothing but hunting ever read it
        // back. So a settlement that could not put a roof up went on trying
        // to for fifteen thousand ticks, and one whose thirsty men were
        // nowhere near water asked for it a hundred and thirty thousand times.
        //
        // A drive that offers something this agent has learned does not work
        // stands aside and lets the next drive have the turn. It is a
        // slackening rather than a ban - see `Lessons::NEVER_QUITE_GIVES_UP` -
        // so a man who has failed at something forty times still tries it now
        // and again, which is how he finds out the world has changed.
        let answer = self.what_this_drive_offers(drive_type, agent, agent_position)?;

        // And judged where it stands, not in the abstract. What an agent has
        // worked out about the circumstances moves this either way: a thing
        // that mostly fails is still worth doing on the afternoon it works,
        // and a thing that mostly pays is not worth doing on the afternoon it
        // does not - see `Lessons::how_likely_to_try_this_here`. Where the
        // agent has worked out nothing, which is every agent to begin with,
        // this is exactly the flat belief it was before.
        // And a gather with nothing to gather is refused before the turn is
        // spent on it rather than after - see
        // `could_this_gather_come_to_anything`.
        if let Action::Gather { resource_type } = &answer {
            if !self.could_this_gather_come_to_anything(agent, agent_position, resource_type) {
                return None;
            }
        }

        if agent
            .lessons
            .will_try_this_here(&crate::agents::Agent::what_was_tried(&answer), here)
        {
            Some(answer)
        } else {
            None
        }
    }

    pub(in crate::analytics) fn what_this_drive_offers(
        &self,
        drive_type: DriveType,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> Option<Action> {
        match drive_type {
            // Water first of the two, always, because it runs out first - but
            // that is now decided by the clocks in `how_hard_it_presses`
            // rather than written down here
            DriveType::Thirst => {
                self.water_action(agent, agent_position, agent.state.is_dehydrated())
            }

            // Eat what is carried, go and get what is not, and failing both
            // stand in a river or go after an animal
            DriveType::Hunger => {
                let starving = agent.state.is_starving() || agent.nutrition.is_starving();
                self.food_action(agent, agent_position, starving)
                    // A store within reach beats a walk out to a berry bush,
                    // which is the whole of what digging one buys.
                    //
                    // It stays *behind* the ordinary food branch, which was
                    // measured both ways. In front, the store is drawn on
                    // five times as often and the rot in the pits halves -
                    // and it costs a fifth of all the food anybody eats and
                    // six of the people in a settlement, because a meal out
                    // of a hole costs two turns where a berry costs one, and
                    // because everything taken out was put back in by
                    // somebody a day earlier. Efficiency did not move.
                    // See ISSUES_FOUND #43.
                    .or_else(|| self.something_out_of_the_store(agent, agent_position))
                    .or_else(|| self.fishing_action(agent, agent_position))
                    .or_else(|| self.hunting_action(agent, agent_position))
            }

            DriveType::Rest => {
                if agent.fatigue.is_sleeping {
                    None
                } else if let Some(clean) = self.somewhere_that_does_not_stink(agent_position) {
                    // "Waste should smell unpleasant and repulse the agents."
                    // Nobody lies down in it. This is the repulsion: a man who
                    // wants to sleep and is standing on a midden moves off it
                    // first, which over a settlement's life is what puts the
                    // midden at the edge of the camp rather than in it.
                    Some(Action::Move { target: clean })
                } else {
                    Some(Action::Sleep { duration: 10 })
                }
            }

            // Being out of harm's way, and only while there is harm about
            DriveType::Safety => {
                let threatened = agent.surroundings.predator_near
                    || agent.surroundings.recently_hurt;

                if threatened && self.nearest_shelter_from(agent_position).is_some() {
                    Some(Action::SeekShelter)
                } else {
                    None
                }
            }

            // Everything that puts food on next year's table
            DriveType::Sustenance => self
                .cooking_action(agent, agent_position)
                .or_else(|| self.muck_action(agent, agent_position))
                .or_else(|| self.farming_action(agent, agent_position))
                .or_else(|| self.transplanting_action(agent, agent_position))
                .or_else(|| self.moving_on(agent, agent_position))
                .or_else(|| self.fishing_action(agent, agent_position)),

            // A coat is shelter you carry, and it comes before walking to a
            // roof: an agent that goes indoors every time it feels the wind
            // never gets around to dressing itself. Walking to a roof is worth
            // a turn only when the weather is actually doing something.
            DriveType::Shelter => self
                .clothing_action(agent, agent_position, true)
                .or_else(|| {
                    let worth_going_in = agent.needs_shelter()
                        || agent.body_temperature.is_too_cold()
                        || agent.surroundings.foul_weather;

                    if worth_going_in && !agent.surroundings.under_shelter {
                        self.nearest_shelter_from(agent_position)
                            .map(|_| Action::SeekShelter)
                    } else {
                        None
                    }
                })
                .or_else(|| self.clothing_action(agent, agent_position, false)),

            // A roof, when there is anything to make one from
            DriveType::Construction => self.raising_a_roof(agent, agent_position),

            // Looking after somebody of your own
            DriveType::Protection => self.protective_action(agent, agent_position),

            // Children, and only when this agent could actually have one and
            // expects to be able to feed it. Without this an agent proposes to
            // the empty air a third of every life - it was the single
            // commonest thing anybody did.
            DriveType::Reproduction => {
                if !agent.should_attempt_reproduction() {
                    return None;
                }
                self.somebody_to_have_a_child_with(agent, agent_position)
                    .map(|them| Action::Mate {
                        target_agent_id: them,
                    })
            }

            // Making a thing needs something to make it out of, and a step
            // that the makings in the pack will actually carry. Asking for a
            // wooden axe was asking for a technology these people have not
            // got: every one of those turns came back
            // `missing technology 'wooden_tools'`.
            // Reducing first, then assembling: a core has to be broken before
            // there is a flake to haft. What each of these wants in the hand
            // is the matrix's business and is enforced before it runs.
            // Working whatever is in the pack into whatever it makes, first.
            //
            // I moved this to the *bottom* of the chain on the reasoning that
            // being equipped ought to come before pottering, since it is
            // undirected and nearly always answerable. Measured, that cost a
            // settlement **two thirds of its vessels** (t = -4.6) and put its
            // rot up (t = 2.1), and the reason is worth writing down: the
            // undirected working is *where bowls come from*. `carve:wood` is a
            // working, so the pottering is the only route to a vessel anybody
            // actually takes. Demoting it did not redirect the turns, it
            // deleted the thing they were producing.
            //
            // Reverted. What survives from that attempt is the two things that
            // were good on their own terms and are kept below: not naming a
            // step that cannot be taken, and getting the hammerstone out
            // before trying to use it.
            DriveType::Utility => agent
                .what_i_would_work_on()
                .map(|(verb, to)| Action::Work { verb, to })
                .or_else(|| {
                    agent
                        .what_i_have_to_work_with(crate::agents::SkillType::Crafting)?;
                    agent
                        .what_vessel_i_would_rather_have()
                        .map(|(verb, to)| Action::Work {
                            verb: verb.to_string(),
                            to: to.to_string(),
                        })
                })
                .or_else(|| {
                    agent
                        .what_i_would_make(
                            self.nearest_fire_from(agent_position, Self::FIRE_REACH, true)
                                .is_some(),
                        )
                        .map(|item_type| Action::Craft { item_type })
                })
                // Something to carry water in. This belongs here, beside the
                // tools, because it is the same kind of thing: a thing a
                // person would rather have than not.
                //
                // Nothing in this world had ever wanted one. `what_i_would_make`
                // asks only after tools - something to hunt with, to cut wood
                // with, to work a hide with - so a bowl and a fired pot both
                // declared what they hold and neither was ever made by
                // anybody. No agent could carry water, so every drink was a
                // walk to the river; and `Boil` was refused for want of
                // something to hold the sea in two hundred and fifty times a
                // world, which put salt out of reach on the same account.
                //
                // The first cut of this put it at the head of the provisioning
                // branch, where it cost a settlement half its winter store and
                // tripled its refused turns - an agent that wanted a bowl and
                // had nothing to carve with returned a refused `Work` every
                // turn instead of burying or drying anything. A branch that
                // can refuse must not stand in front of branches that cannot.
                .or_else(|| {
                    // Somebody standing here with the thing, who wants what is
                    // going spare. A trade is quicker than a walk.
                    self.somebody_to_trade_with(agent, agent_position)
                        .map(|with| Action::Trade { with })
                })
                // And a thing already lying on the ground is quicker than
                // either: stooping is where scavenging belongs, beside going
                // out to fetch a thing rather than ahead of making one out of
                // what is already in the pack.
                .or_else(|| self.something_worth_stooping_for(agent, agent_position))
                // And failing all of that, somebody standing here who has it.
                // Last, because it is the answer nobody reaches for first.
                .or_else(|| {
                    self.somebody_to_take_from(agent, agent_position)
                        .map(|from| Action::TakeFrom { from })
                })
                .or_else(|| {
                    agent
                        .what_i_must_find()
                        .map(|resource_type| Action::Gather { resource_type })
                }),

            // Putting something by needs something to put by
            // Putting something by, which until now could not mean food.
            // `what_i_can_spare` explicitly excludes anything anybody eats,
            // and the only place to put anything was a global bag of counts
            // with no position that nothing ever spoiled in - so a settlement
            // stored materials it rarely needed and never once stored a meal.
            // A hole in the cold ground is what a people this far along has.
            DriveType::Preparedness => self
                .putting_food_by(agent, agent_position)
                .or_else(|| {
                    agent.what_i_can_spare().map(|(what, how_many)| Action::Store {
                        item_type: what,
                        amount: how_many,
                    })
                }),

            // Nothing in the world is fine enough to want yet - see
            // ISSUES_FOUND.md #5. Until something is, this need has no answer
            // and stands aside rather than spending the turn walking after a
            // resource that does not exist.
            DriveType::Luxury => None,

            // A curious man picks up the bright stone he walked past.
            //
            // Nothing else in the model ever puts iron in a pack: no drive
            // asks for it, because nobody yet knows what it is for. It gets
            // picked up because it glitters, which is the only way anybody
            // ever came to be holding one at a fire.
            DriveType::Curiosity => {
                // First, doing again the thing he has just worked out how to
                // do. There is no use in mind for what comes out of it; that
                // is the point. Nobody can want a metal knife until somebody
                // has held a metal blade, and nobody holds one until somebody
                // does the trick a second time for its own sake.
                // Something growing underfoot that nobody has ever tried.
                // This is where a people's larder comes from and where some
                // of its people go: the only way to find out whether a plant
                // is food is for somebody to eat one.
                if let Some(action) = self.tasting_action(agent, agent_position) {
                    return Some(action);
                }

                // Turning over something in the pack that might be for
                // something. Cheaper than any other experiment - it costs the
                // turn and nothing else.
                if let Some(what) = agent.what_i_would_look_at() {
                    return Some(Action::Examine { what });
                }

                // Asking somebody about a thing of theirs you have never
                // seen the like of.
                //
                // First, because it is the cheapest way anybody ever finds
                // anything out - somebody else has already spent the season
                // finding it out the hard way - and because it is the only
                // route a discovery has ever had out of the head that made it.
                if let Some((who, what)) =
                    self.somebody_to_ask_about_something(agent, agent_position)
                {
                    return Some(Action::AskAbout { who, what });
                }

                // What happens if I leave this here.
                //
                // The oldest kind of experiment there is and the only one in
                // this model whose answer does not arrive in the turn it was
                // asked: put a thing down, remember what it was like, and walk
                // back in a day or two to see what became of it - see
                // `who_came_back_to_look`.
                if let Some(what) = agent.what_i_would_leave_out() {
                    return Some(Action::PutDown { what });
                }

                // Going to get a handful of something nobody here has ever
                // done anything with.
                //
                // Every material in the chain is gathered by somebody who
                // already wants the thing it makes - and nobody can want the
                // thing until somebody has made one. Clay had been spawning
                // on every riverbank in every world since the project began
                // and no agent had ever picked any of it up, so
                // `ResourceType::Clay`, `Pottery` and `Bricks` were three
                // enum variants with nothing whatever behind them.
                //
                // Curiosity is the right drive for it: this is fetching a
                // material for no reason except that nobody has tried it.
                if let Some(what) = self.something_nobody_has_tried_within_reach(agent, agent_position)
                {
                    return Some(Action::Gather { resource_type: what });
                }

                // Doing something to a thing to see what it turns into. The
                // cheapest kind of experiment there is: the materials are in
                // the pack and the tool is in the hand either way.
                let a_fire_is_to_hand = self
                    .nearest_fire_from(agent_position, Self::FIRE_REACH, true)
                    .is_some();
                if let Some((verb, to)) = agent.what_working_i_would_try_out(a_fire_is_to_hand) {
                    return Some(Action::Work { verb, to });
                }

                // And putting the wrong thing where a part goes, which is
                // how a people gets past the things it already knows how to
                // make. Rare, because it costs the materials whether or not
                // anything comes of it.
                let feeling_experimental = {
                    use rand::Rng;
                    crate::core::dice::roll().gen_bool(Self::HOW_OFTEN_ANYBODY_TRIES_A_SWAP)
                };

                if feeling_experimental {
                    if let Some((instead_of_making, instead_of, put_in)) =
                        agent.what_i_would_swap()
                    {
                        return Some(Action::TrySwapping {
                            instead_of_making,
                            instead_of,
                            put_in,
                        });
                    }
                }

                if let Some(what) = agent.what_i_would_try_out() {
                    // The conditions are checked here and not only in the
                    // executor, because an agent that keeps asking for a thing
                    // it cannot do here learns not to ask for it at all - see
                    // `Lessons::will_try_this_again` - and would give up on
                    // metalworking after a dozen turns spent away from a fire.
                    let wants_a_fire = crate::environment::making::how_to_make(&what)
                        .is_some_and(|step| step.over_a_fire);

                    let can_do_it_here = !wants_a_fire
                        || self
                            .nearest_fire_from(agent_position, Self::FIRE_REACH, true)
                            .is_some();

                    if can_do_it_here {
                        return Some(Action::Craft { item_type: what });
                    }
                }

                let agent_has_none = agent.how_many_i_have("iron") < 2;
                if agent_has_none && agent.have_i_seen("iron") {
                    Some(Action::Gather {
                        resource_type: "iron".to_string(),
                    })
                } else {
                    Some(Self::generate_action_for_drive(drive_type, agent_position))
                }
            }

            // Company needs somebody to keep it
            DriveType::Social => {
                if !agent.surroundings.company {
                    return None;
                }

                // Handing somebody something they need is the plainest sociable
                // act there is, and it costs the giver, which is what makes it
                // one. It comes before talking because a gift says more.
                if let Some(to) = self.somebody_to_give_to(agent, agent_position) {
                    return Some(Action::GiveTo { to });
                }

                Some(Self::generate_action_for_drive(drive_type, agent_position))
            }

            // The rest keep the simple mapping they had
            other => Some(Self::generate_action_for_drive(other, agent_position)),
        }
    }
}
