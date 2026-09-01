// src/analytics/between_us/threat.rs
//! Fear, anger, and the four answers to a thing in the way.
//!
//! What a threat comes to when it is added up, whether there is anywhere to
//! run, how poor the way out is, and which of fight, flee, freeze and stand
//! this one settles on. A beast counts as somebody here: what matters is that
//! it is not you and it is in the way.
//!
//! Part of how one agent stands towards another - see [`super`].

use super::super::Simulation;
use crate::environment::Action;

impl Simulation {
    /// Head off in the opposite direction, far enough not to arrive back where
    /// you started worrying.
    pub(in crate::analytics) fn put_ground_between(from: (i32, i32, i32), away_from: (i32, i32)) -> Action {
        let dx = from.0 - away_from.0;
        let dy = from.1 - away_from.1;

        // Standing on top of it, which should not happen, is still a reason to
        // be somewhere else
        let span = ((dx * dx + dy * dy) as f32).sqrt();
        let (dx, dy, span) = if span < 1.0 { (1, 0, 1.0) } else { (dx, dy, span) };

        let far = Self::FAR_ENOUGH_AWAY as f32;
        Action::Move {
            target: (
                from.0 + (dx as f32 / span * far) as i32,
                from.1 + (dy as f32 / span * far) as i32,
                from.2,
            ),
        }
    }


    /// How much an agent has to resent one particular person before it will do
    /// anything about it.
    ///
    /// Read per person rather than off the total, because `should_attack` sums
    /// every source: three mild grudges of 0.2 read as a man ready to fight,
    /// and there is nobody he is actually ready to fight.
    pub(in crate::analytics) const ENOUGH_TO_ROUND_ON_SOMEBODY: f32 = 0.5;

    /// Square up to the people you resent, or shrink from them.
    ///
    /// A grudge is the reason; whether it comes out as standing up or backing
    /// down is the same appraisal a wolf gets. Measured before this existed,
    /// anger at people ran at 0.806 for every agent that read as ready to
    /// fight and anger at creatures at 0.025 - so nearly all the anger in the
    /// model was a grudge against somebody, held against them for life,
    /// decaying at one per cent a tick and with no way to be acted on at all.
    ///
    /// The grudge itself is not touched. Only which feeling it comes out as.
    pub(in crate::analytics) fn square_up_to_the_people_i_resent(&mut self) {
        let standing: Vec<(uuid::Uuid, (i32, i32, i32), f32, bool)> = self
            .population
            .agents
            .iter()
            .map(|agent| {
                (
                    agent.id,
                    agent.state.position,
                    agent.own_strength(),
                    agent.state.is_alive,
                )
            })
            .collect();

        for index in 0..self.population.agents.len() {
            let (mine, from) = {
                let agent = &self.population.agents[index];
                if !agent.state.is_alive {
                    continue;
                }
                (agent.own_strength(), agent.state.position)
            };

            let resented: Vec<(uuid::Uuid, f32)> = {
                let agent = &self.population.agents[index];
                agent
                    .emotions
                    .anger_at_people()
                    .into_iter()
                    .filter(|(_, held)| *held >= Self::ENOUGH_TO_ROUND_ON_SOMEBODY)
                    .collect()
            };

            for (who, held) in resented {
                let Some((_, where_they_are, theirs, alive)) =
                    standing.iter().copied().find(|(id, ..)| *id == who)
                else {
                    continue;
                };
                if !alive {
                    continue;
                }

                let paces = (where_they_are.0 - from.0)
                    .abs()
                    .max((where_they_are.1 - from.1).abs());

                let agent = &mut self.population.agents[index];

                // Out of sight is not out of mind - the grudge stands - but
                // there is nothing to shrink from, and leaving the fear
                // standing would keep the agent running from an empty field
                // and keep it below the bar for ever squaring up to anybody.
                if paces > Self::CLOSE_ENOUGH_TO_WORRY_ABOUT {
                    agent
                        .emotions
                        .set_fear(crate::agents::EmotionSource::Agent(who), 0.0);
                    continue;
                }

                if theirs > mine {
                    // You cannot take them. What you feel about them is the
                    // same; what you will do about it is get out of the way.
                    let nearness = 1.0
                        - (paces as f32 / (Self::CLOSE_ENOUGH_TO_WORRY_ABOUT as f32 + 1.0));
                    agent.emotions.set_fear(
                        crate::agents::EmotionSource::Agent(who),
                        held * nearness,
                    );
                } else {
                    // You can, so it stays anger and stays where it was
                    agent
                        .emotions
                        .set_fear(crate::agents::EmotionSource::Agent(who), 0.0);
                }
            }
        }
    }

    /// Turn on the person you resent, if they are within arm's reach.
    ///
    /// Gated on the grudge against that one person rather than on total anger,
    /// and on the agent reckoning it can take them - which the appraisal above
    /// has already decided by turning the hopeless cases into fear.
    pub(in crate::analytics) fn round_on_whoever_angers_me(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> Option<Action> {
        let (who, held) = agent.emotions.who_angers_me_most()?;
        if held < Self::ENOUGH_TO_ROUND_ON_SOMEBODY {
            return None;
        }

        let them = self
            .population
            .agents
            .iter()
            .find(|other| other.id == who && other.state.is_alive)?;

        // Nobody raises a hand to a child, and nobody to their own parent
        if them.state.life_stage == crate::agents::LifeStage::Infant
            || them.state.life_stage == crate::agents::LifeStage::Child
            || them.parent_ids.contains(&agent.id)
            || agent.parent_ids.contains(&who)
        {
            return None;
        }

        let paces = (them.state.position.0 - agent_position.0)
            .abs()
            .max((them.state.position.1 - agent_position.1).abs());
        if paces > Self::HUNT_REACH {
            return None;
        }

        Some(Action::Attack {
            target_agent_id: who,
            weapon: agent.equipment.get_weapon().map(|held| held.name.clone()),
        })
    }

    /// Get away from the person you are afraid of.
    pub(in crate::analytics) fn run_from_whoever_frightens_me(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> Option<Action> {
        let (who, _) = agent.emotions.who_frightens_me_most()?;
        let them = self
            .population
            .agents
            .iter()
            .find(|other| other.id == who && other.state.is_alive)?;

        let where_they_are = them.state.position;
        let paces = (where_they_are.0 - agent_position.0)
            .abs()
            .max((where_they_are.1 - agent_position.1).abs());
        if paces > Self::CLOSE_ENOUGH_TO_WORRY_ABOUT {
            return None;
        }

        Some(Self::put_ground_between(
            agent_position,
            (where_they_are.0, where_they_are.1),
        ))
    }

    /// The whole answer to a thing that would kill you, in the order the
    /// specification gives it.
    ///
    /// > if this threat seems like something he can overcome, the man attacks.
    /// > if not, the man flees in fear. if fleeing does not seem like an
    /// > option, then the only alternative is to fight. if fighting does not
    /// > seem like an option, then the only alternative is to flee. if the
    /// > agent cannot select between one of those two options, they freeze.
    ///
    /// The appraisal has already answered the first question: it comes out as
    /// anger where the thing can be overcome and fear where it cannot - see
    /// `Agent::appraise_what_is_there`. What was missing was everything after
    /// it. An agent who could not overcome the thing ran, and if there was
    /// nowhere to run it simply went back to gathering berries with a wolf at
    /// its elbow; an agent who could overcome it fought, and if its arms were
    /// gone it did the same. Neither of the two cornered cases existed, and
    /// nor did the third answer.
    ///
    /// A wrapper over [`Self::what_this_threat_comes_to`] that drops the branch
    /// name. Nothing in the live path calls it - the tree itself is called
    /// directly, because the tally wants the name - so this exists for the
    /// tests, which ask the tree a question and do not care which branch
    /// answered. Two names for one question; see ISSUES_FOUND.md #99.
    pub(in crate::analytics) fn how_this_one_answers_a_threat(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> Option<Action> {
        self.what_this_threat_comes_to(agent, agent_position).1
    }

    /// The thing to get away from, and where it is.
    ///
    /// The fear drive needs this and the fight-or-flee tree did not provide
    /// it: that tree runs off what the agent has *named* in its emotions, and
    /// an agent can be plainly frightened - something with teeth eight paces
    /// off - without having named anything yet. So fear had nothing to run
    /// from and offered `SeekShelter` or nothing at all.
    ///
    /// Nearest rather than worst, because what you run from is what is close.
    pub(in crate::analytics) fn what_to_run_from(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> Option<Action> {
        let here = (agent_position.0, agent_position.1);

        let closest = self
            .world
            .animals
            .get_in_radius(here, Self::HOW_FAR_A_FRIGHT_CARRIES)
            .into_iter()
            .filter(|animal| animal.is_alive() && !animal.is_domesticated)
            .filter(|animal| {
                // Only what this one could not face. A hare has an
                // `attack_damage` above nought and is not a reason to run.
                self.world
                    .animals
                    .get_species(&animal.species_id)
                    .map(|species| {
                        species.attack_damage > 0.0
                            && !agent.could_i_fight_at_all(species.attack_damage)
                    })
                    .unwrap_or(false)
            })
            .min_by_key(|animal| {
                (animal.position.0 - here.0).abs() + (animal.position.1 - here.1).abs()
            })?;

        Some(Action::FleeFrom {
            away_from: (closest.position.0, closest.position.1, agent_position.2),
        })
    }

    /// How far off something has to be before it stops being worth running
    /// from.
    pub(in crate::analytics) const HOW_FAR_A_FRIGHT_CARRIES: f32 = 12.0;

    /// What this one has to hand for a fight, by name.
    ///
    /// From `environment::making`, which is the vocabulary the model actually
    /// stocks. It used to come from `agent.equipment`, which nothing has ever
    /// put a weapon into, so the field was `None` in every fight this model
    /// has ever run - see ISSUES_FOUND.md #100.
    pub(in crate::analytics) fn what_is_in_hand_for_this(
        agent: &crate::agents::Agent,
    ) -> Option<String> {
        agent
            .what_i_have_to_work_with(crate::agents::skills::SkillType::Hunting)
            .map(|tool| tool.called.to_string())
    }

    /// The same tree, and the name of the branch it came out of.
    ///
    /// Every way of declining used to look like `None` from outside, which is
    /// why #66 could measure `Freeze` at zero in sixty-four worlds and say
    /// nothing about whether the tree was working or idle. The name is what
    /// `Simulation::what_a_threat_came_to` counts.
    pub(in crate::analytics) fn what_this_threat_comes_to(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> (&'static str, Option<Action>) {
        // What it is, where it is, and how hard it hits
        let named = agent
            .emotions
            .what_frightens_me_most()
            .map(|(kind, _)| (kind, false))
            .or_else(|| {
                agent
                    .emotions
                    .what_angers_me_most()
                    .map(|(kind, _)| (kind, true))
            });

        // Frightened or angry enough to act, and not at any creature. A
        // grudge against a neighbour comes out here: the branches below this
        // one deal with people.
        let Some((kind, standing)) = named else {
            return ("nothing named", None);
        };

        // It named something that is not about. The feeling outlasts the
        // thing by however long the decay takes.
        let Some((which, where_it_is, paces)) = self.nearest_of_kind(kind, agent_position) else {
            return ("named, but not about", None);
        };

        let coming = self
            .world
            .animals
            .get_all()
            .iter()
            .find(|animal| animal.id == which)
            .and_then(|animal| self.world.animals.get_species(&animal.species_id))
            .map(|species| species.attack_damage)
            .unwrap_or(0.0);

        // Standing your ground is something you do to what is in front of
        // you. Nobody crosses a field to pick a fight with a wolf, and an
        // agent that is not afraid of a thing four paces off has no business
        // with it at all - it gets on with its day.
        if standing && paces > Self::WITHIN_A_STEP_OR_TWO {
            return ("not worth crossing to", None);
        }

        let could_fight = agent.could_i_fight_at_all(coming);

        // Somebody of this agent's own, in the way of the thing, who cannot
        // deal with it themselves. A person does not run from a wolf that is
        // standing over their child, whatever the odds are - and the odds are
        // exactly what this sets aside. It is the one place in the model
        // where an agent knowingly takes the worse of two options.
        if could_fight && self.somebody_of_mine_is_in_the_way(agent, where_it_is, coming) {
            return (
                "stands over one of its own",
                Some(if paces <= Self::HUNT_REACH {
                    Action::Fight {
                        animal_id: which,
                        weapon: Self::what_is_in_hand_for_this(agent),
                    }
                } else {
                    Action::Move {
                        target: (where_it_is.0, where_it_is.1, agent_position.2),
                    }
                }),
            );
        }

        let could_run = agent.could_i_run_at_all(Self::WHAT_RUNNING_COSTS)
            && self.is_there_anywhere_to_run(
                &agent.exploration_knowledge,
                agent_position,
                where_it_is,
            );

        let fight = || {
            if paces <= Self::HUNT_REACH {
                Action::Fight {
                    animal_id: which,
                    weapon: Self::what_is_in_hand_for_this(agent),
                }
            } else {
                // Close the last pace or two, and no further
                Action::Move {
                    target: (where_it_is.0, where_it_is.1, agent_position.2),
                }
            }
        };

        let run = || Action::FleeFrom {
            away_from: (where_it_is.0, where_it_is.1, agent_position.2),
        };

        match (standing, could_fight, could_run) {
            // What it wanted to do, and it can
            (true, true, _) => ("stands its ground", Some(fight())),
            (false, _, true) => ("runs", Some(run())),

            // Cornered: it wanted to run and there is nowhere to go, so it
            // turns and fights. Or it wanted to fight and cannot lift an arm,
            // so it goes.
            (false, true, false) => ("cornered, so fights", Some(fight())),
            (true, false, true) => ("cannot fight, so runs", Some(run())),

            // Neither. This is the case the decision never had an answer for.
            (_, false, false) => ("freezes", Some(Action::Freeze)),
        }
    }

    /// Whether somebody this agent loves is in the way of the thing, and could
    /// not deal with it themselves.
    ///
    /// The paradigm case, and the reason this exists: a wolf standing over a
    /// child. The child cannot fight it and very likely cannot outrun it, and
    /// the parent is the only thing between them. What comes of that is a
    /// fight the parent may well lose, which is the point - the specification
    /// asks for agents that can lay down their lives for their family, and an
    /// agent that only ever fights what it can beat cannot do that.
    pub(in crate::analytics) fn somebody_of_mine_is_in_the_way(
        &self,
        agent: &crate::agents::Agent,
        where_it_is: (i32, i32),
        coming: f32,
    ) -> bool {
        self.population
            .agents
            .iter()
            .filter(|them| them.id != agent.id && them.state.is_alive)
            .filter(|them| {
                agent
                    .relationships
                    .get_relationship(&them.id)
                    .is_some_and(|bond| bond.is_loved_one())
            })
            // In the way of it: nearer the thing than a person would choose
            // to be
            .filter(|them| {
                (them.state.position.0 - where_it_is.0)
                    .abs()
                    .max((them.state.position.1 - where_it_is.1).abs())
                    <= Self::STANDING_OVER_THEM
            })
            // And unable to do anything about it. Somebody who can fight it
            // themselves is not being protected, they are being joined
            .any(|them| !them.could_i_fight_at_all(coming))
    }

    /// How near the thing somebody has to be before they count as being in
    /// its way.
    pub(in crate::analytics) const STANDING_OVER_THEM: i32 = 2;

    /// Whether there is any ground to run to, away from the thing.
    ///
    /// Half the answer to "fleeing does not seem like an option": a man with
    /// his back to a cliff has nowhere to go however much he would like to.
    /// The other half is the body, and belongs to the agent - see
    /// `Agent::could_i_run_at_all`.
    ///
    /// It asks the running itself, rather than asking the same question in
    /// its own words. It used to have its own: three ways out at three
    /// paces, where the running tried three ways out at nineteen. A man
    /// three paces from a shoreline with the thing inland has somewhere to
    /// go at three paces and nothing but water at nineteen, so the decision
    /// said run and the running said there was nowhere to run - and nothing
    /// about the next turn was different, so it said it again. One measured
    /// world produced 76,644 of those refusals, three quarters of every turn
    /// taken in the settlement. Two vocabularies for one question is the
    /// recurring defect; this is the fifth time it has cost something.
    pub(in crate::analytics) fn is_there_anywhere_to_run(
        &self,
        remembers: &crate::agents::exploration::ExplorationKnowledge,
        from: (i32, i32, i32),
        away_from: (i32, i32),
    ) -> bool {
        self.where_this_one_would_run(remembers, from, away_from)
            .is_some()
    }

    /// Where a frightened person actually goes, if anywhere.
    ///
    /// Eight ways out rather than three, and each of them tried at the full
    /// bolt first and then at every shorter distance down to a single pace.
    /// Both of those are the same point: the ways out that exist are not
    /// always the ways out somebody would choose, and a person hemmed in by
    /// water on three sides does not stand still because the gap is narrow.
    /// Behind is in the list too - running past the thing is a poor answer,
    /// and the scoring says so, but it beats being caught standing.
    pub(in crate::analytics) fn where_this_one_would_run(
        &self,
        remembers: &crate::agents::exploration::ExplorationKnowledge,
        from: (i32, i32, i32),
        away_from: (i32, i32),
    ) -> Option<(i32, i32, i32)> {
        let dx = from.0 - away_from.0;
        let dy = from.1 - away_from.1;
        let span = (((dx * dx + dy * dy) as f32).sqrt()).max(1.0);

        let straight = (dx as f32 / span, dy as f32 / span);

        // Straight away, then an eighth-turn either side, then a quarter,
        // and so round to behind. Listed nearest-to-away first, so that
        // where the scoring cannot separate two landings the one that was
        // asked about first wins.
        let ways = [0i32, 1, -1, 2, -2, 3, -3, 4].map(|eighths| {
            let (sin, cos) = (eighths as f32 * std::f32::consts::FRAC_PI_4).sin_cos();
            (
                straight.0 * cos - straight.1 * sin,
                straight.0 * sin + straight.1 * cos,
            )
        });

        let bolt = Self::HOW_FAR_A_FRIGHTENED_PERSON_GETS;

        ways.iter()
            .filter_map(|(wx, wy)| {
                // The furthest this way goes. Getting clear is the point of
                // running, so a short bolt is a fallback and not a choice.
                (1..=bolt).rev().find_map(|paces| {
                    let landed = (
                        (from.0 as f32 + wx * paces as f32).round() as i32,
                        (from.1 as f32 + wy * paces as f32).round() as i32,
                        from.2,
                    );

                    let moved = landed.0 != from.0 || landed.1 != from.1;

                    (moved && self.is_passable_tile(landed.0, landed.1)).then_some(landed)
                })
            })
            .min_by(|one, other| {
                self.how_poor_a_way_out(remembers, from, away_from, *one)
                    .partial_cmp(&self.how_poor_a_way_out(remembers, from, away_from, *other))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    }

    /// What is wrong with running that way.
    ///
    /// Two things, and they pull against each other: what this one remembers
    /// happening where it would land, and how much ground it puts between
    /// itself and the thing. Running headlong into the wood where the pack
    /// lives is how somebody gets away from one animal and into four.
    pub(in crate::analytics) fn how_poor_a_way_out(
        &self,
        remembers: &crate::agents::exploration::ExplorationKnowledge,
        from: (i32, i32, i32),
        away_from: (i32, i32),
        landed: (i32, i32, i32),
    ) -> f32 {
        let bad = remembers.how_bad_is_it_there(
            crate::world::Position::new(landed.0, landed.1),
            self.current_tick,
        );

        let off = |where_it_is: (i32, i32)| {
            let dx = (where_it_is.0 - away_from.0) as f32;
            let dy = (where_it_is.1 - away_from.1) as f32;
            (dx * dx + dy * dy).sqrt()
        };

        let gained =
            (off((landed.0, landed.1)) - off((from.0, from.1))) / Self::HOW_FAR_A_FRIGHTENED_PERSON_GETS as f32;

        bad - Self::WHAT_GETTING_CLEAR_IS_WORTH * gained.clamp(-1.0, 1.0)
    }

    /// What a full bolt's worth of ground is worth against a place somebody
    /// remembers going badly.
    ///
    /// Less than the worst thing that can be remembered and more than the
    /// least, which is the whole of what it has to be: a wood a man was
    /// mauled in is not worth running into to gain nineteen paces, and a
    /// field he once went hungry in is not worth staying put for.
    pub(in crate::analytics) const WHAT_GETTING_CLEAR_IS_WORTH: f32 = 0.5;

    /// What share of what an agent has left counts as having got off lightly
    pub(in crate::analytics) const A_SCRATCH: f32 = 0.25;

    /// Feel about whatever is standing between an agent and what it needs.
    ///
    /// The specification, in two questions. Does a thing threaten my ability
    /// to satisfy my drives - and if so, can I fight it? Did a thing prevent
    /// it - and if so, can I fight *that*? Where the answer is yes it comes
    /// out as anger and the agent stands its ground; where it is no it comes
    /// out as fear and the agent goes.
    ///
    /// `ThreatAssessment` has always turned coping potential into one or the
    /// other, and `respond_to_threat` has always called it. What was missing
    /// was anything to call `respond_to_threat` except the resolution of a
    /// blow that had already landed: a wolf ten paces off and closing
    /// produced no feeling at all until it bit somebody. Measured over three
    /// worlds, mean fear ran at 0.01 to 0.06 and mean anger at exactly zero,
    /// and not one agent in a hundred and seventy ever reached the 0.6 that
    /// `should_flee` wants - so the branch of `generate_action` that lets an
    /// agent run or fight never once fired.
    pub(in crate::analytics) fn feel_about_what_stands_in_the_way(&mut self) {
        // What is out there, and how much of a match each one is
        let hunters: Vec<((i32, i32), f32, String)> = self
            .world
            .animals
            .get_all()
            .iter()
            .filter(|animal| animal.is_alive() && !animal.is_domesticated)
            .filter_map(|animal| {
                let species = self.world.animals.get_species(&animal.species_id)?;

                // What a thing does to somebody who has done nothing to it.
                // Reading this off `attack_damage` said a rabbit was a threat,
                // because a rabbit will bite you if you pick it up - and once
                // several of a thing began adding up, a herd of reindeer came
                // to about a wolf and the settlement stopped hunting.
                let temper = species.behavior.how_much_it_menaces_you();
                if temper <= 0.0 || species.attack_damage <= 0.0 {
                    return None;
                }

                // What it is worth in a fight, on the same scale an agent
                // reckons itself on: a healthy body, and what it can do with it
                let condition = (animal.current_health / species.health.max(1.0)).clamp(0.0, 1.0);
                let menace = (species.attack_damage / 20.0).clamp(0.1, 2.0) * temper;

                Some((
                    (animal.position.0, animal.position.1),
                    condition * menace,
                    species.name.clone(),
                ))
            })
            .collect();

        for agent in self.population.agents.iter_mut() {
            if !agent.state.is_alive {
                continue;
            }

            let (x, y, _) = agent.state.position;

            // Everything within sight of this agent that would eat it. All of
            // it, not the worst of it: the appraisal used to take the single
            // largest thing in view and throw the rest away, so a man
            // surrounded by four wolves faced whichever one happened to be
            // nearest and felt no differently about it than he would about
            // one.
            let closing: Vec<(f32, &String)> = hunters
                .iter()
                .filter_map(|((hx, hy), strength, what)| {
                    let paces = (hx - x).abs().max((hy - y).abs());
                    if paces > Self::CLOSE_ENOUGH_TO_WORRY_ABOUT {
                        return None;
                    }

                    // A wolf across the field is not the wolf at your elbow.
                    // Without this an agent felt the full weight of anything
                    // within ten paces, and spent a third of its life angry at
                    // something it could barely see.
                    let nearness = 1.0
                        - (paces as f32 / (Self::CLOSE_ENOUGH_TO_WORRY_ABOUT as f32 + 1.0));

                    Some((strength * nearness, what))
                })
                .collect();

            // What it is called is the name of the worst of them: a man
            // hemmed in by wolves is afraid of wolves, whatever else is in
            // the field
            let worst = closing
                .iter()
                .max_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal))
                .map(|(_, what)| (*what).clone());

            match worst {
                Some(what) => {
                    let all: Vec<f32> = closing.iter().map(|(strength, _)| *strength).collect();
                    let pack = crate::agents::ThreatAssessment::a_pack_of(&all);

                    agent.appraise_what_is_there(
                        pack,
                        crate::agents::EmotionSource::Creature(what),
                    );
                }
                None => {
                    // Nothing is stalking this one, so whatever it was
                    // frightened of has gone
                    agent.emotions.nothing_is_stalking_me();
                }
            }
        }
    }
}
