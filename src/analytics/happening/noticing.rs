// src/analytics/happening/noticing.rs
//! What a person finds out by being somewhere when something happens.
//!
//! A bright stone in a fire, clay that went hard, a thing laid out that dried
//! - and going back the next day to see whether it is still true.
//!
//! Part of what happens whether or not anybody decides anything - see
//! [`super`]. Called from [`crate::analytics::turn`], in the order argued over
//! there.

use super::super::Simulation;
use crate::agents::practices::Circumstance;
use crate::agents::wondering::Kept;
use log::debug;

impl Simulation {
    /// How often a cook leaves food on the fire too long, by how practised
    /// they are.
    ///
    /// Deliberately gentler than the generic `SkillCategory::failure_chance`,
    /// which is calibrated for botching an axe: burning a meal is a smaller
    /// and commoner mistake, and a fifty-fifty campfire would make cooking not
    /// worth attempting.
    pub(in crate::analytics) fn burn_chance(cooking_level: i32) -> f32 {
        match cooking_level {
            level if level <= -6 => 0.20, // has never done it before
            -5..=-1 => 0.10,
            0..=5 => 0.04,
            _ => 0.0, // years of it
        }
    }

    /// What to call food that has come off a fire.
    ///
    /// `id_to_item_type` reads through these prefixes, so cooked fish is still
    /// fish to everything that asks what it is.
    pub(in crate::analytics) fn prepared_item_id(item_id: &str, cooked_well: bool) -> String {
        let base = crate::agents::storage_integration::base_item_id(item_id);

        if cooked_well {
            format!("cooked_{}", base)
        } else {
            format!("burnt_{}", base)
        }
    }

    /// How often a man in exactly the right position works out what he is
    /// looking at.
    ///
    /// Set so that finding out is a thing that happens to a settlement over
    /// seasons rather than to an individual over an afternoon: a curious agent
    /// with the makings in his hands and a fire in front of him needs of the
    /// order of a hundred turns of standing there.
    pub(in crate::analytics) const HOW_OFTEN_ANYBODY_WORKS_IT_OUT: f64 = 0.01;

    /// Nobody works anything out while they are frightened or starving.
    pub(in crate::analytics) const CURIOUS_ENOUGH_TO_NOTICE: f32 = 0.25;

    /// Somebody, somewhere, finds out how to do something new.
    ///
    /// This is the specification's "rock + fire = ?": the outcome of putting
    /// two things together is not apparent until the conditions are right, and
    /// then it is apparent all at once. Nothing here is a plan - an agent
    /// cannot want a metal knife before anybody has seen metal - it is the
    /// accident of standing in the right place holding the right things while
    /// curious enough to be paying attention.
    pub(in crate::analytics) fn somebody_notices_something(&mut self) {
        use rand::Rng;

        let mut rng = crate::core::dice::roll();
        let mut found: Vec<(usize, &'static str)> = Vec::new();

        for (index, agent) in self.population.agents.iter().enumerate() {
            if !agent.state.is_alive {
                continue;
            }

            let curiosity = agent
                .drives
                .get(crate::core::DriveType::Curiosity)
                .map(|drive| drive.value)
                .unwrap_or(0.0);
            if curiosity < Self::CURIOUS_ENOUGH_TO_NOTICE {
                continue;
            }

            let holding = |what: &str| agent.how_many_i_have(what);

            for step in crate::environment::making::everything_to_find_out() {
                if agent.knows_how_to(step) || !step.makings_to_hand(&holding) {
                    continue;
                }
                if let Some(wanted) = step.wants_in_hand {
                    if agent.how_many_i_have(wanted) == 0 {
                        continue;
                    }
                }
                if step.over_a_fire
                    && self
                        .nearest_fire_from(agent.state.position, Self::FIRE_REACH, true)
                        .is_none()
                {
                    continue;
                }

                // A practised hand notices sooner, because it knows what it
                // is looking at.
                let odds = Self::HOW_OFTEN_ANYBODY_WORKS_IT_OUT
                    * curiosity as f64
                    * agent.skills.hand_for(step.hands) as f64;

                if rng.gen_bool(odds.clamp(0.0, 1.0)) {
                    found.push((index, step.makes));
                    break;
                }
            }
        }

        for (index, what) in found {
            if self.population.agents[index].found_out_how_to(what) {
                debug!(
                    "Agent {} worked out how to make {what}",
                    self.population.agents[index].id
                );
            }
        }
    }

    /// How often a man who has the thought at all follows it through.
    ///
    /// Ten times the rate of stumbling on something over a fire
    /// (`HOW_OFTEN_ANYBODY_WORKS_IT_OUT`), and deliberately: this is not an
    /// accident, it is a man who already knows the job looking at a thing of a
    /// kind with what he uses for it. The hard part - knowing the technique,
    /// knowing the material, having the two in mind at once - has already
    /// happened by the time this rolls.
    pub(in crate::analytics) const HOW_OFTEN_A_LIKENESS_IS_FOLLOWED_UP: f64 = 0.1;

    /// **One use of a material teaches its siblings.**
    ///
    /// The innovation path. A man who knaps ordinary stone into a tip, and who
    /// can point to a bank of flint, works out that flint will do it too - and
    /// better. He does not need the flint in his hand. He needs to know the
    /// job, and to know there is a thing of that kind to be had.
    ///
    /// That is what makes it different from `somebody_notices_something`,
    /// which is the accident of standing over a fire with the right things in
    /// your pack. This one is reasoning: two things a man already knows, put
    /// beside each other. The specification calls it the innovation path and
    /// it is what makes a craft grow rather than merely be inherited.
    ///
    /// **And it is what reads the name on the map.** A `SpatialMemory` carries
    /// `what_it_is` - what was there, where the rememberer knew what it was
    /// for - and until now nothing asked. Knowing where the flint is is the
    /// whole of the prompt: a settlement that has never walked past a flint
    /// bank never has the thought, and one that walks past it every day has it
    /// within a season. See `core::memory` and ISSUES_FOUND #194.
    ///
    /// A thing in the pack counts too. Picking a stone up and turning it over
    /// is at least as good a prompt as remembering where it lies, and refusing
    /// it would make the rule about maps rather than about likeness.
    pub(in crate::analytics) fn somebody_puts_two_and_two_together(&mut self) {
        use crate::environment::making::Making;
        use rand::Rng;

        // The candidates first, and **not a die touched until there is one.**
        //
        // The stream is shared with the whole world, so a pass that rolls
        // whether or not it has anything to decide re-shuffles every seeded
        // outcome downstream of it - the ecology included. It knocked over
        // `the_land_will_only_carry_so_many`, which is ten sheep on a quarter
        // of a square kilometre and thin enough already, and no sheep had
        // died differently: the draws had simply moved. Deciding nothing
        // should cost nothing.
        let mut could_have_the_thought: Vec<(usize, &'static Making, f64)> = Vec::new();

        for (index, agent) in self.population.agents.iter().enumerate() {
            if !agent.state.is_alive {
                continue;
            }

            let curiosity = agent
                .drives
                .get(crate::core::DriveType::Curiosity)
                .map(|drive| drive.value)
                .unwrap_or(0.0);
            if curiosity < Self::CURIOUS_ENOUGH_TO_NOTICE {
                continue;
            }

            for step in crate::environment::making::everything_to_find_out() {
                if agent.knows_how_to(step) {
                    continue;
                }

                // Is this the same job, done with a thing of a kind?
                let a_likeness = crate::environment::making::every_way_to_make(step.makes)
                    .filter(|known| agent.knows_how_to(known))
                    .any(|known| {
                        step.needs.iter().any(|(wants, _)| {
                            known.needs.iter().any(|(has, _)| {
                                crate::environment::making::are_they_of_a_kind(wants, has)
                            })
                        })
                    });

                if !a_likeness {
                    continue;
                }

                // And can he point to the stuff? In his pack, or on his map.
                //
                // **The map half of this never fires today, and it is worth
                // knowing why.** Flint is not a thing that lies in the
                // ground - there is no `ResourceType::Flint`, it is made by
                // smashing a stone core - so no sight pass can write it and
                // no map can carry it. Measured over 8 worlds of 180 days,
                // 5 of them worked flint knapping out and not one of the 96
                // people who ever lived had flint on his map: every man who
                // had the thought was holding the stuff.
                //
                // The one sibling that *does* lie in the ground is cotton,
                // and it is on maps in quantity - so this arm would be
                // load-bearing the moment `LASHING_FROM_COTTON` became a
                // discovery, which is the run that cost 6 of 64 settlements
                // their first winter. The cheap sibling is made rather than
                // found and the found one is too dear. See ISSUES_FOUND #195.
                let to_be_had = step.needs.iter().any(|(wants, _)| {
                    agent.how_many_i_have(wants) > 0
                        || agent
                            .memory
                            .spatial_memories
                            .iter()
                            .any(|place| place.what_i_could_name_it() == Some(*wants))
                });

                if !to_be_had {
                    continue;
                }

                let odds = Self::HOW_OFTEN_A_LIKENESS_IS_FOLLOWED_UP
                    * curiosity as f64
                    * agent.skills.hand_for(step.hands) as f64;

                could_have_the_thought.push((index, step, odds));
                break;
            }
        }

        if could_have_the_thought.is_empty() {
            return;
        }

        let mut rng = crate::core::dice::roll();
        let mut worked_out: Vec<(usize, &'static str)> = Vec::new();

        for (index, step, odds) in could_have_the_thought {
            if rng.gen_bool(odds.clamp(0.0, 1.0)) {
                worked_out.push((index, step.makes));
            }
        }

        for (index, what) in worked_out {
            if self.population.agents[index].found_out_how_to(what) {
                debug!(
                    "Agent {} worked out that something else would make {what}",
                    self.population.agents[index].id
                );
            }
        }
    }

    /// How far a man will walk to get off fouled ground.
    ///
    /// Not far. The point is to step off the midden, not to leave the country.
    pub(in crate::analytics) const OFF_THE_MIDDEN: i32 = 3;

    /// What nobody can carry any more goes on the ground.
    ///
    /// A pack refuses what will not fit, so a load can never be *put* over
    /// the limit. It gets there the other way: `max_weight` is worked out
    /// fresh every turn from what the body can lift and what it has to carry
    /// things in, and both fall. A man loads up in his strong summer, goes
    /// hungry, weakens, and wakes carrying more than he can hold - and
    /// nothing in this model ever noticed. Measured across eight worlds, an
    /// autumn pack held **38.9 units against a capacity of 26.0**, and
    /// because a pack that is already over its limit refuses everything, the
    /// load was frozen there for the rest of the man's life. He could never
    /// pick up food again: **97% of autumn agent-ticks had not room for a
    /// single handful**, and 27,968 units of food a year went back on the
    /// bush while six thousand stood ripe on the ground.
    ///
    /// So this is an invariant rather than a decision. What a person cannot
    /// carry is not carried, and it is not destroyed either - it stays where
    /// they were standing, to be picked up by them or by anybody else. The
    /// heaviest thing goes first, and food goes last of all: a man walking
    /// under a load he cannot manage puts the stone down before the supper.
    /// See ISSUES_FOUND.md #126.
    pub(in crate::analytics) fn what_nobody_can_carry_any_more(&mut self) {
        use crate::world::Position;

        let now = self.current_tick;

        for index in 0..self.population.agents.len() {
            if !self.population.agents[index].state.is_alive {
                continue;
            }

            if self.population.agents[index].how_much_too_much_i_am_carrying() <= 0.0 {
                continue;
            }

            let here = {
                let at = self.population.agents[index].state.position;
                Position::new(at.0, at.1)
            };

            // Each pass takes the heaviest thing that is not food, a tool in
            // use or the pack itself, and only as much of it as the shortfall
            // wants. A stack is rarely enough on its own - a body half as
            // much again over its limit is not put right by three sticks -
            // so the next pass takes the next heaviest, and it ends when
            // there is room or when there is nothing left that anybody would
            // put down.
            while self.population.agents[index].how_much_too_much_i_am_carrying() > 0.0 {
                let Some(what) = self.population.agents[index].what_i_would_set_down() else {
                    break;
                };

                let how_many = self.population.agents[index]
                    .how_much_of_this_i_would_set_down(&what);

                let Some(mut down) = self.population.agents[index]
                    .inventory
                    .get_item(&what)
                    .cloned()
                else {
                    break;
                };
                down.quantity = how_many;

                self.population.agents[index]
                    .inventory
                    .remove_item(&what, how_many);
                self.world.somebody_left_this(down, here, now);

                debug!(
                    "Agent {} could not carry {how_many} {what} and set it down",
                    self.population.agents[index].id
                );
            }
        }
    }

    /// Whoever is near enough to a question they left open goes and looks.
    ///
    /// This is the half of "what happens if" that no other kind of curiosity
    /// in this model has: the answer arrives days later and somewhere else,
    /// and somebody has to be standing there to get it. What is learned is
    /// learned from the change — the meat has gone off, the strips have dried,
    /// the clay is not clay any more — and it is recorded against the
    /// circumstances the thing was *left* in rather than the ones it is found
    /// in, because the rain that ruined it has usually stopped by then.
    ///
    /// A question that is never answered is also an answer. Four days on, a
    /// thing exactly as it was left teaches that leaving that thing about
    /// comes to nothing, and the agent stops doing it - which is the whole
    /// difference between an experiment and a habit.
    pub(in crate::analytics) fn who_came_back_to_look(&mut self) {
        let now = self.current_tick;

        for index in 0..self.population.agents.len() {
            if !self.population.agents[index].state.is_alive {
                continue;
            }

            if self.population.agents[index].wonderings.is_empty() {
                continue;
            }

            let standing = self.population.agents[index].state.position;

            // What is answerable this tick, worked out with the world borrowed
            // and the agent not.
            let mut answers: Vec<(String, bool, Vec<Circumstance>, Option<&'static str>)> =
                Vec::new();
            let mut done: Vec<usize> = Vec::new();

            for (which, wondering) in self.population.agents[index].wonderings.iter().enumerate() {
                let near = (standing.0 - wondering.where_it_is.x)
                    .abs()
                    .max((standing.1 - wondering.where_it_is.y).abs());

                let close_enough = near
                    <= crate::agents::wondering::Wondering::CLOSE_ENOUGH_TO_GO_AND_LOOK;

                // Where to go and look depends on what was done. Burying
                // puts a thing in a hole and salting leaves it in the pack;
                // only leaving it out puts it on the grass.
                let as_it_is = match wondering.where_to_look() {
                    Kept::OnTheGround => self
                        .world
                        .what_is_lying_at(&wondering.where_it_is)
                        .into_iter()
                        .find(|left| {
                            left.item.item_id == wondering.what
                                || left.item.item_id != wondering.as_it_was.called
                        })
                        .map(|left| crate::agents::wondering::Watched::of(&left.item)),
                    Kept::InThePit => self
                        .world
                        .pit_at(wondering.where_it_is)
                        .and_then(|pit| {
                            pit.holds
                                .iter()
                                .find(|item| item.item_id == wondering.what)
                        })
                        .map(crate::agents::wondering::Watched::of),
                    // In the pack, which goes where its owner goes - so this
                    // one is always answerable and never wants a walk back.
                    Kept::InMyPack => self.population.agents[index]
                        .inventory
                        .get_item(&wondering.what)
                        .map(crate::agents::wondering::Watched::of),
                };

                let can_see_it = close_enough
                    || wondering.where_to_look() == Kept::InMyPack;
                let waited = wondering.given_up_on(now);

                match (can_see_it, as_it_is) {
                    (true, Some(as_it_is)) => {
                        // What the verb makes of it - and the verb decides,
                        // because a buried thing that has not changed is the
                        // whole point of burying it and a thing left on the
                        // grass that has not changed is nothing at all.
                        if let Some(became) = wondering.what_it_means(&as_it_is, waited) {
                            answers.push((
                                wondering.called(),
                                became.for_the_better,
                                wondering.in_this.clone(),
                                Some(became.says),
                            ));
                            done.push(which);
                        }
                    }
                    // Somebody walked off with it, it was eaten, or it rotted
                    // away to nothing. No answer, and none to be had.
                    (true, None) => done.push(which),
                    (false, _) => {
                        if waited {
                            done.push(which);
                        }
                    }
                }
            }

            if answers.is_empty() && done.is_empty() {
                continue;
            }

            let agent = &mut self.population.agents[index];

            for (called, for_the_better, in_this, says) in answers {
                agent
                    .lessons
                    .record_particular_here(&called, for_the_better, &in_this);

                if let Some(says) = says {
                    debug!("Agent {} came back and found {called}: {says}", agent.id);
                }

                *self.what_anybody_found_out.entry(called).or_insert(0) += 1;
            }

            for which in done.into_iter().rev() {
                agent.wonderings.remove(which);
            }
        }
    }

    /// Clay left lying at a lit fire is not clay in the morning.
    ///
    /// The ember accident that already existed is somebody *carrying* clay
    /// while they sit at a fire, and it happens to them rather than being
    /// done. This is the deliberate version, and it is what makes "what
    /// happens if I put clay in the fire" a question anybody can actually put
    /// - the answer arrives a few days later at the place it was left, like
    /// every other question of that kind.
    pub(in crate::analytics) fn what_the_fire_hardened(&mut self) {
        let now = self.current_tick;
        let mut hardened: Vec<crate::world::Position> = Vec::new();

        for which in 0..self.world.dropped.len() {
            let left = &self.world.dropped[which];

            if left.item.item_id != crate::agents::Agent::THE_ONE_MATERIAL_A_FIRE_CHANGES {
                continue;
            }

            if now.saturating_sub(left.since) < Self::HOW_LONG_THE_FIRE_TAKES_TO_HARDEN_IT {
                continue;
            }

            let where_it_is = left.where_it_is;
            let at_a_fire = self
                .nearest_fire_from(
                    (where_it_is.x, where_it_is.y, 0),
                    Self::WITHIN_REACH_OF_THE_HEARTH,
                    true,
                )
                .is_some();

            if !at_a_fire {
                continue;
            }

            let how_many = self.world.dropped[which].item.quantity;
            self.world.dropped[which].item = crate::agents::InventoryItem::new_container(
                "stoneware".to_string(),
                how_many,
                crate::environment::making::WHAT_A_FIRED_POT_HOLDS,
            );

            hardened.push(where_it_is);
        }

        // And whoever is near enough to see it saw it.
        for where_it_was in hardened {
            for agent in self.population.agents.iter_mut() {
                if !agent.state.is_alive {
                    continue;
                }

                let paces = (agent.state.position.0 - where_it_was.x)
                    .abs()
                    .max((agent.state.position.1 - where_it_was.y).abs());

                if paces <= Self::CLOSE_ENOUGH_TO_SEE_IT_COME_UP {
                    agent.found_out_how_to(Self::THAT_FIRE_HARDENS_CLAY);
                }
            }
        }
    }

    /// How long a lump has to sit in the embers before it comes out hard.
    ///
    /// A day. Long enough that it is something the fire did rather than
    /// something that happened, short enough that somebody who left it there
    /// on purpose is still about to see it.
    pub(in crate::analytics) const HOW_LONG_THE_FIRE_TAKES_TO_HARDEN_IT: u32 =
        crate::environment::seasons::TICKS_PER_DAY;

    pub(in crate::analytics) fn what_the_embers_did(&mut self) {
        use rand::Rng;

        if self.current_tick % Self::HOW_OFTEN_THE_EMBERS_ARE_ASKED != 0 {
            return;
        }

        let mut rng = crate::core::dice::roll();
        let mut hardened: Vec<crate::world::Position> = Vec::new();

        for index in 0..self.population.agents.len() {
            {
                let agent = &self.population.agents[index];
                if !agent.state.is_alive || agent.how_many_i_have("clay") == 0 {
                    continue;
                }
            }

            let stood = self.population.agents[index].state.position;
            if self
                .nearest_fire_from(stood, Self::WITHIN_REACH_OF_THE_HEARTH, true)
                .is_none()
            {
                continue;
            }

            if !rng.gen_bool(Self::HOW_OFTEN_A_LUMP_FINDS_THE_EMBERS) {
                continue;
            }

            let agent = &mut self.population.agents[index];
            agent.inventory.remove_item("clay", 1);
            agent.inventory.add_item(crate::agents::InventoryItem::new_container(
                "stoneware".to_string(),
                1,
                crate::environment::making::WHAT_A_FIRED_POT_HOLDS,
            ));

            hardened.push(crate::world::Position::new(stood.0, stood.1));
        }

        // And whoever was sitting round the same fire saw it happen.
        for where_it_was in hardened {
            for agent in self.population.agents.iter_mut() {
                if !agent.state.is_alive {
                    continue;
                }

                let paces = (agent.state.position.0 - where_it_was.x)
                    .abs()
                    .max((agent.state.position.1 - where_it_was.y).abs());

                if paces > Self::CLOSE_ENOUGH_TO_SEE_IT_COME_UP {
                    continue;
                }

                if agent.found_out_how_to(Self::THAT_FIRE_HARDENS_CLAY) {
                    debug!("Agent {} saw clay come out of a fire hard", agent.id);
                }
                agent.lessons.record_particular("fire:claypot", true);
            }
        }
    }

    /// How often anybody's fire is asked about.
    pub(in crate::analytics) const HOW_OFTEN_THE_EMBERS_ARE_ASKED: u32 = crate::environment::seasons::TICKS_PER_DAY;

    /// And how often a day at a fire with clay in the pack costs a lump of it.
    ///
    /// Rare. This is meant to happen once or twice in a settlement's life and
    /// then never matter again, because after it has happened once somebody
    /// knows and can do it on purpose.
    pub(in crate::analytics) const HOW_OFTEN_A_LUMP_FINDS_THE_EMBERS: f64 = 0.02;

    /// What a lump of clay coming out of a fire hard teaches.
    ///
    /// The same name the working that does it deliberately makes, so that
    /// having seen it is the same thing as knowing how - see
    /// `making::FIRE_A_POT`.
    pub const THAT_FIRE_HARDENS_CLAY: &'static str = "stoneware";

    /// What an agent has to have seen before it will deliberately lay food
    /// out to dry.
    pub const THAT_LAYING_IT_OUT_KEEPS_IT: &'static str =
        crate::agents::Agent::THAT_LAYING_IT_OUT_KEEPS_IT;
}
