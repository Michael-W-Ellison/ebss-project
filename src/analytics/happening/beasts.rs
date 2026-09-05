// src/analytics/happening/beasts.rs
//! The beasts, which have two drives worth the name: eat, and do not be
//! eaten.
//!
//! What they make of us, what they do about it, and what happens when a hungry
//! one decides we are the answer.
//!
//! Part of what happens whether or not anybody decides anything - see
//! [`super`]. Called from [`crate::analytics::turn`], in the order argued over
//! there.

use super::super::Simulation;
use log::debug;

impl Simulation {
    /// The nearest living animal of a named kind, and how far off it is.
    ///
    /// An agent's fear and anger are held against a species name rather than
    /// against a particular animal - it is afraid of wolves, not of wolf #4 -
    /// so acting on the feeling means finding which wolf it can see.
    pub(in crate::analytics) fn nearest_of_kind(
        &self,
        kind: &str,
        from: (i32, i32, i32),
    ) -> Option<(uuid::Uuid, (i32, i32), i32)> {
        self.world
            .animals
            .get_all()
            .iter()
            .filter(|animal| animal.is_alive() && !animal.is_domesticated)
            .filter_map(|animal| {
                let species = self.world.animals.get_species(&animal.species_id)?;
                if species.name != kind {
                    return None;
                }

                let paces = (animal.position.0 - from.0)
                    .abs()
                    .max((animal.position.1 - from.1).abs());
                if paces > Self::CLOSE_ENOUGH_TO_WORRY_ABOUT {
                    return None;
                }

                Some((animal.id, animal.position, paces))
            })
            .min_by_key(|(_, _, paces)| *paces)
    }

    /// What the beasts make of us.
    ///
    /// The simplified other half of `feel_about_what_stands_in_the_way`. An
    /// animal has two drives worth the name - eat, and do not be eaten - and
    /// this is the second: run from what you cannot beat, turn on what you
    /// can. `AnimalState::Fleeing` and `AnimalState::Attacking` have been in
    /// the model since the model had animals and nothing had ever set either
    /// of them, so a deer stood placidly in a field while somebody walked up
    /// to it with a spear.
    ///
    /// Temper decides how kindly the odds get read, and a Passive thing never
    /// stands its ground however the arithmetic comes out - a rabbit that
    /// fights a wolf is not a rabbit.
    pub(in crate::analytics) fn what_the_beasts_make_of_us(&mut self) {
        use crate::environment::fauna::AnimalState;

        // Everybody who might be a threat to something, and what they are
        // worth in a fight
        let people: Vec<((i32, i32), f32, uuid::Uuid)> = self
            .population
            .agents
            .iter()
            .filter(|agent| agent.state.is_alive)
            .map(|agent| {
                let armed = agent
                    .what_i_have_to_work_with(crate::agents::SkillType::MeleeCombat)
                    .is_some();

                let worth = Self::WHAT_A_PERSON_IS_WORTH_TO_A_BEAST
                    * if armed { Self::WHAT_A_SPEAR_ADDS } else { 1.0 };

                (
                    (agent.state.position.0, agent.state.position.1),
                    worth,
                    agent.id,
                )
            })
            .collect();

        // And the beasts, which are a threat to each other
        let beasts: Vec<((i32, i32), f32, uuid::Uuid, bool)> = self
            .world
            .animals
            .get_all()
            .iter()
            .filter(|animal| animal.is_alive())
            .filter_map(|animal| {
                let species = self.world.animals.get_species(&animal.species_id)?;
                let worth = Self::what_a_beast_is_worth_in_a_fight(
                    animal.current_health,
                    species.health,
                    species.attack_damage,
                );
                let hunts = species.behavior.how_much_it_menaces_you() >= 1.0;
                Some((animal.position, worth, animal.id, hunts))
            })
            .collect();

        let mut made_up_their_minds: Vec<(uuid::Uuid, AnimalState)> = Vec::new();

        for animal in self.world.animals.get_all().iter() {
            if !animal.is_alive() || !animal.is_wild() {
                continue;
            }

            let Some(species) = self.world.animals.get_species(&animal.species_id) else {
                continue;
            };

            let mine = Self::what_a_beast_is_worth_in_a_fight(
                animal.current_health,
                species.health,
                species.attack_damage,
            );

            // The worst thing within sight of it: a person, or something
            // bigger than itself that eats meat
            let from_people = people
                .iter()
                .map(|(at, worth, who)| (*at, *worth, *who))
                .filter(|(at, _, _)| Self::within(*at, animal.position, Self::AS_FAR_AS_A_BEAST_LOOKS));

            let from_beasts = beasts
                .iter()
                .filter(|(_, _, who, hunts)| *hunts && *who != animal.id)
                .map(|(at, worth, who, _)| (*at, *worth, *who))
                .filter(|(at, _, _)| Self::within(*at, animal.position, Self::AS_FAR_AS_A_BEAST_LOOKS));

            let worst = from_people
                .chain(from_beasts)
                .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

            let Some((where_it_is, coming, who)) = worst else {
                continue;
            };

            // Temper reads the odds. A rabbit never fights.
            let nerve = species.behavior.how_readily_it_stands_its_ground();
            let stands = nerve > 0.0 && mine * nerve >= coming * Self::WHAT_IT_TAKES_TO_TURN_AND_FACE;

            made_up_their_minds.push((
                animal.id,
                if stands {
                    AnimalState::Attacking { target_id: who }
                } else {
                    AnimalState::Fleeing {
                        from_position: where_it_is,
                    }
                },
            ));
        }

        for (which, made_up) in made_up_their_minds {
            if let Some(animal) = self
                .world
                .animals
                .get_all_mut()
                .iter_mut()
                .find(|animal| animal.id == which)
            {
                animal.state = made_up;
                animal.state_timer = Self::HOW_LONG_A_BEAST_KEEPS_ITS_NERVE;
            }
        }
    }

    /// And then they do something about it.
    ///
    /// Fleeing puts ground between the animal and whatever it saw. Standing
    /// its ground keeps it where it is - what happens next is whoever came at
    /// it getting bitten, which the hunt already handles.
    pub(in crate::analytics) fn the_beasts_act_on_it(&mut self) {
        use crate::environment::fauna::AnimalState;

        let width = self.world.grid.width as i32;
        let height = self.world.grid.height as i32;

        let mut bolted: Vec<(uuid::Uuid, (i32, i32))> = Vec::new();

        for animal in self.world.animals.get_all().iter() {
            let AnimalState::Fleeing { from_position } = animal.state else {
                continue;
            };

            if !animal.is_alive() {
                continue;
            }

            let dx = animal.position.0 - from_position.0;
            let dy = animal.position.1 - from_position.1;
            let span = (((dx * dx + dy * dy) as f32).sqrt()).max(1.0);

            let bolt = Self::HOW_FAR_A_FRIGHTENED_BEAST_GETS as f32;
            let landed = (
                (animal.position.0 as f32 + dx as f32 / span * bolt) as i32,
                (animal.position.1 as f32 + dy as f32 / span * bolt) as i32,
            );

            let landed = (
                landed.0.clamp(0, width - 1),
                landed.1.clamp(0, height - 1),
            );

            if self.is_passable_tile(landed.0, landed.1) {
                bolted.push((animal.id, landed));
            }
        }

        for (which, to) in bolted {
            if let Some(animal) = self
                .world
                .animals
                .get_all_mut()
                .iter_mut()
                .find(|animal| animal.id == which)
            {
                animal.position = to;
                animal.use_stamina(Self::WHAT_BOLTING_COSTS_A_BEAST);
            }
        }
    }

    /// What a beast is worth in a fight, on the same scale everything else is
    /// reckoned on: how sound it is, and what it can do with that.
    pub(in crate::analytics) fn what_a_beast_is_worth_in_a_fight(health: f32, full: f32, damage: f32) -> f32 {
        let condition = (health / full.max(1.0)).clamp(0.0, 1.0);
        condition * (damage / 20.0).clamp(0.1, 2.0)
    }

    /// A hungry predator turns on the people.
    ///
    /// Nothing in the model let an animal touch an agent: predation was
    /// animal-on-animal only, so a wolf could starve beside a settlement. A
    /// predator that is merely hungry keeps to the herds; one that is close
    /// to starving takes what it can reach, and that includes an agent.
    ///
    /// This is where thinning the herds comes back on the settlement that did
    /// it. Agents hunt for skins, the herds go down, the predators go hungry,
    /// and hungry predators come looking.
    pub(in crate::analytics) fn process_predator_attacks(&mut self) {
        self.predators_try_their_luck(None);
    }

    /// The same, against one person only.
    ///
    /// For the minute clock: somebody in danger takes their turn a minute at a
    /// time, and the thing that is after them has to get its minutes too, or
    /// fleeing would be free. A man who moves out of reach in the first two
    /// minutes is not struck at in the other twenty-eight; one who is cornered
    /// is struck at every minute he stays there. That is the chase resolving
    /// inside the turn rather than a single roll standing for half an hour of
    /// it. See `Simulation::everybody_takes_a_turn`.
    pub(in crate::analytics) fn predators_try_their_luck_at(&mut self, who: usize) {
        self.predators_try_their_luck(Some(who));
    }

    fn predators_try_their_luck(&mut self, only: Option<usize>) {
        use rand::Rng;

        let mut rng = crate::core::dice::roll();
        let current_tick = self.current_tick;

        // Who is where, and who is desperate enough to try
        let agent_positions: Vec<(usize, (i32, i32))> = self
            .population
            .agents
            .iter()
            .enumerate()
            .filter(|(_, agent)| agent.state.is_alive)
            .filter(|(index, _)| only.is_none_or(|who| *index == who))
            .map(|(index, agent)| (index, (agent.state.position.0, agent.state.position.1)))
            .collect();

        if agent_positions.is_empty() {
            return;
        }

        let mut strikes: Vec<(uuid::Uuid, usize, f32, f32)> = Vec::new();

        for animal in self.world.animals.get_all() {
            if !animal.is_alive() || animal.is_domesticated || !animal.is_hungry() {
                continue;
            }

            let species = match self.world.animals.get_species(&animal.species_id) {
                Some(species) => species,
                None => continue,
            };

            if species.prey_species.is_empty() || species.attack_damage <= 0.0 {
                continue;
            }

            // Nearest agent within striking distance
            let target = agent_positions
                .iter()
                .filter(|(_, position)| {
                    (position.0 - animal.position.0).abs() <= Self::PREDATOR_STRIKE_RANGE
                        && (position.1 - animal.position.1).abs() <= Self::PREDATOR_STRIKE_RANGE
                })
                .min_by_key(|(_, position)| {
                    (position.0 - animal.position.0).abs()
                        + (position.1 - animal.position.1).abs()
                });

            let (agent_index, _) = match target {
                Some(target) => target,
                None => continue,
            };

            // A full belly makes a cautious animal. Hunger is what changes
            // its mind, and only really at the end of it.
            let pressure =
                ((animal.hunger / animal.max_hunger.max(1.0)) - 0.5).clamp(0.0, 0.5) / 0.5;
            let odds = 0.01 + pressure * pressure * 0.14;

            if rng.gen::<f32>() < odds {
                strikes.push((
                    animal.id,
                    *agent_index,
                    species.attack_damage,
                    species.food_value * 0.25,
                ));
            }
        }

        for (animal_id, agent_index, damage, fed) in strikes {
            // Standing there while something bites you is a fight, and how it
            // goes is what the agent takes away from it. This is where the
            // record mostly comes from: agents seldom set upon one another,
            // but the country is full of things that will try them.
            {
                use crate::agents::practices::Undertaking;

                // Winning is driving the thing off with a scratch; losing is
                // being mauled and living. Reckoning it as "did the blow kill
                // me" made every survivor a winner, which is half a lesson:
                // nobody ever learned that running was the better idea,
                // because everyone who learned it was dead.
                let agent = &mut self.population.agents[agent_index];
                let came_off_well = damage < agent.state.health * Self::A_SCRATCH;
                agent.lessons.record(Undertaking::Fighting, came_off_well);
            }

            {
                // What is in the hand when the thing comes at you. A man who
                // gets a spear between himself and a wolf takes a good deal
                // less of it than a man who gets an arm up, and this is the
                // whole of what the matrix means by a verb that wants a tool:
                // `defend with` cannot be done bare-handed, so a man with
                // nothing in his hands simply does not do it.
                let landed = self.population.agents[agent_index].what_a_blow_costs_me(damage);
                let turned = damage - landed;

                let agent = &mut self.population.agents[agent_index];

                // And putting a shaft in the way of something is hard on the
                // shaft
                if turned > 0.0 {
                    if let Some(broke) =
                        agent.wear_what_i_worked_with(crate::agents::SkillType::MeleeCombat)
                    {
                        debug!("Agent {} broke a {broke} keeping it off", agent.id);
                    }
                }

                agent.take_damage(landed);
                agent.emotions.record_attack(animal_id, current_tick);

                debug!(
                    "Agent {} was attacked by a hungry animal ({landed:.0} of {damage:.0} damage got through)",
                    agent.id
                );
            }

            // Even a glancing blow is something in the stomach
            if let Some(animal) = self.world.animals.get_mut(&animal_id) {
                animal.feed(fed);
            }
        }
    }
}
