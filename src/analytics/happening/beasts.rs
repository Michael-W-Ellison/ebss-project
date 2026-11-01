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
    /// What a grown person weighs, in kilogrammes, as a beast reckons prey.
    pub(in crate::analytics) const WHAT_A_GROWN_PERSON_WEIGHS: f32 = 60.0;

    /// How many times a beast's own weight a person can be before it will
    /// not go for them. Four: a fifteen-kilogramme dog will; an eagle will
    /// not.
    pub(in crate::analytics) const HOW_MUCH_BIGGER_A_MAN_CAN_BE: f32 = 4.0;

    /// From what age, in years, a person is too big for a small hunter.
    pub(in crate::analytics) const TOO_BIG_FOR_A_SMALL_HUNTER: u32 = 12;

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
        let beasts: Vec<((i32, i32), f32, uuid::Uuid, bool, String)> = self
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
                // **What menaces a person is not what hunts a beast.** This
                // was temper alone, so a mammoth - `Territorial`, and a
                // danger to anybody who walks up to it - was a threat to
                // every animal in sight of it, its own herd included. Every
                // mammoth faced down the one beside it, turn after turn,
                // stood in `Attacking` for the whole of its life, and never
                // grazed: twenty-three on the big map, none born, all starved
                // (#283). A boar did the same three turns in four. A beast is
                // a threat to another beast when it eats meat, which is what
                // the line below has always said it was looking for.
                let hunts = species.behavior.how_much_it_menaces_you() >= 1.0
                    && species.where_it_sits() != crate::environment::TrophicRole::PrimaryConsumer;
                Some((animal.position, worth, animal.id, hunts, animal.species_id.clone()))
            })
            // Only what hunts is a threat to another beast. Every animal
            // looked through every other for the few that hunt, which with
            // three thousand head on the big map was nine million looks a
            // turn. Dropping the rest here keeps the hunters in the order
            // they were, so the worst is chosen exactly as it was. See
            // ISSUES_FOUND #306.
            .filter(|(_, _, _, hunts, _)| *hunts)
            .collect();

        // Where in the list each one stands, kept with what it decided, so the
        // deciding is applied without looking anybody up. See ISSUES_FOUND
        // #306.
        let mut made_up_their_minds: Vec<(usize, AnimalState)> = Vec::new();

        for (at, animal) in self.world.animals.get_all().iter().enumerate() {
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
                .filter(|(_, _, who, hunts, kind)| {
                    *hunts && *who != animal.id && *kind != animal.species_id
                })
                .map(|(at, worth, who, _, _)| (*at, *worth, *who))
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
                at,
                if stands {
                    AnimalState::Attacking { target_id: who }
                } else {
                    AnimalState::Fleeing {
                        from_position: where_it_is,
                    }
                },
            ));
        }

        for (at, made_up) in made_up_their_minds {
            if let Some(animal) = self.world.animals.get_all_mut().get_mut(at) {
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

        let mut bolted: Vec<(usize, (i32, i32))> = Vec::new();

        for (at, animal) in self.world.animals.get_all().iter().enumerate() {
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
                bolted.push((at, landed));
            }
        }

        for (at, to) in bolted {
            if let Some(animal) = self.world.animals.get_all_mut().get_mut(at) {
                animal.position = to;
                animal.use_stamina(Self::WHAT_BOLTING_COSTS_A_BEAST);
            }
        }
    }

    /// What a beast is worth in a fight, on the same scale everything else is
    /// reckoned on: how sound it is, and what it can do with that.
    /// Whether a beast of this kind would ever strike a person of this age.
    ///
    /// It hunts, its blow does something, and it is not too small to take
    /// them. This is the whole of what makes a beast dangerous here: nothing
    /// else strikes anybody of its own accord. It is asked both where the
    /// beasts strike and where people size them up, so that nobody is
    /// frightened of, or squares up to, a thing that would never touch them
    /// (#327).
    pub(in crate::analytics) fn could_it_take(species: &crate::environment::fauna::AnimalSpecies, years_old: u32) -> bool {
        Self::a_hunter(species)
            && !(Self::too_small_for_the_grown(species) && years_old >= Self::TOO_BIG_FOR_A_SMALL_HUNTER)
    }

    /// Whether a beast of this kind charges anybody who comes too near.
    ///
    /// A territorial beast, or one with a temper that does not live by
    /// hunting: the bear, the mammoth and the boar. They do not come looking,
    /// and nothing they do is about being hungry. They keep their ground, and
    /// somebody who walks into it is struck. A wolf, a lion or an eagle lives
    /// by hunting, keeps away from people while it is fed, and strikes when
    /// it is starving (`could_it_take`); a boar roots for its living, takes a
    /// rabbit when it finds one, and charges a man at close quarters. #327 found that nothing but a hungry
    /// hunter ever struck anybody, took the fear of mammoths away to match,
    /// and left this as "a behaviour of the beasts still to be written".
    /// A cow, an elk or a goat turns on a man who hunts it and on nobody
    /// else (`fights_back`), so a defensive temper is not enough.
    pub(in crate::analytics) fn would_it_charge(species: &crate::environment::fauna::AnimalSpecies) -> bool {
        use crate::environment::fauna::DietType;
        use crate::environment::AnimalBehavior;

        species.attack_damage > 0.0
            && match species.behavior {
                AnimalBehavior::Territorial => true,
                AnimalBehavior::Aggressive => species.diet != DietType::Carnivore,
                _ => false,
            }
    }

    /// Whether a beast of this kind would ever harm a person of this age of
    /// its own accord: by hunting them, or by charging them off its ground.
    /// What people size beasts up by, so that they are wary of exactly what
    /// would strike them.
    pub(in crate::analytics) fn could_it_harm(species: &crate::environment::fauna::AnimalSpecies, years_old: u32) -> bool {
        Self::could_it_take(species, years_old) || Self::would_it_charge(species)
    }

    /// How near a charging beast lets anybody come: twenty metres.
    pub(in crate::analytics) const A_BEASTS_GROUND: i32 = 2;

    /// What beast's ground this cell is in, if any: within twenty metres of
    /// a beast that charges (#372).
    pub(in crate::analytics) fn whose_ground_is_this(&self, at: (i32, i32)) -> Option<String> {
        self.world
            .get_animals_in_radius(at, (Self::A_BEASTS_GROUND + 1) as f32)
            .into_iter()
            .filter(|animal| animal.is_alive() && !animal.is_domesticated && !animal.is_kept())
            .filter(|animal| {
                (animal.position.0 - at.0).abs().max((animal.position.1 - at.1).abs()) <= Self::A_BEASTS_GROUND
            })
            .find_map(|animal| {
                let species = self.world.animals.get_species(&animal.species_id)?;
                Self::would_it_charge(species).then(|| species.name.clone())
            })
    }

    /// Whether a step from here to there walks into a beast's ground: there
    /// is somebody's ground and here is not (#372). Somebody already inside
    /// - the beast came to them - is let out.
    pub(in crate::analytics) fn walks_into_a_beasts_ground(&self, here: (i32, i32), there: (i32, i32)) -> bool {
        self.whose_ground_is_this(there).is_some() && self.whose_ground_is_this(here).is_none()
    }

    /// Whether this one stops short of a beast's ground on the way from here
    /// to there: anybody does, except somebody starving or down to the last
    /// quarter of their water.
    ///
    /// A boar that settled on a camp's hearth kept everybody outside its
    /// ground out of their own camp, and their store with it. The walk home
    /// stopped short and called itself done, so the same walk was decided
    /// again every minute: two people on smooth seed 3 starved three cells
    /// from their hearth, walking at it two hundred and eighty times a day
    /// for two days (#376). Starving, a person goes in and takes the chance.
    ///
    /// And the same at the water: on smooth seed 24 two people died of thirst
    /// walking at a stream three cells off a thousand times a day, a boar's
    /// ground between them and it.
    pub(in crate::analytics) fn does_this_one_stop_short(&self, agent_index: usize, here: (i32, i32), there: (i32, i32)) -> bool {
        let body = &self.population.agents[agent_index].state;
        let desperate = body.is_starving() || body.physiology.hydration <= Self::DESPERATE_FOR_WATER;
        !desperate && self.walks_into_a_beasts_ground(here, there)
    }

    /// How low the water in a body has to run before its owner walks past a
    /// beast to get more: the last of the four bands, where everything the
    /// body does is at a quarter (`Physiology::capability`).
    pub(in crate::analytics) const DESPERATE_FOR_WATER: f32 = 0.25;

    /// And how near before anybody is wary of it: fifty metres. A hunter
    /// worries anybody within `CLOSE_ENOUGH_TO_WORRY_ABOUT`, because it comes
    /// for them; a beast that only keeps its ground worries only somebody
    /// about to walk into it, which is what keeps people from squaring up to
    /// a mammoth across a field (#327).
    pub(in crate::analytics) const NEAR_ENOUGH_TO_BE_WARY_OF_IT: i32 = 5;

    /// How often a beast charges somebody inside its ground, a pass.
    pub(in crate::analytics) const HOW_OFTEN_IT_CHARGES: f32 = 0.25;

    /// It hunts, and its blow does something.
    pub(in crate::analytics) fn a_hunter(species: &crate::environment::fauna::AnimalSpecies) -> bool {
        // A meat-eater. A boar takes a rabbit when it finds one and lives by
        // rooting, and reckoned a hunter for the rabbits it struck at grown
        // people when it was hungry - twenty blows among the twelve dead of
        // blows in forty settlements (#372). It still charges whoever comes
        // into its ground (`would_it_charge`).
        !species.prey_species.is_empty()
            && species.attack_damage > 0.0
            && species.diet == crate::environment::fauna::DietType::Carnivore
    }

    /// Several times smaller than a grown person.
    pub(in crate::analytics) fn too_small_for_the_grown(species: &crate::environment::fauna::AnimalSpecies) -> bool {
        species.mass_kg * Self::HOW_MUCH_BIGGER_A_MAN_CAN_BE < Self::WHAT_A_GROWN_PERSON_WEIGHS
    }

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
        let current_turn = self.current_turn;

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

        // The species as well as the beast, because what a survivor takes
        // away from this is a fear of *wolves*, not of one wolf - and until
        // #212 what went on the record here was the animal's uuid in a field
        // that means "the person who hit me".
        let mut strikes: Vec<(uuid::Uuid, String, usize, f32, f32)> = Vec::new();
        // Those a beast came at and stopped short of, what it was and where
        let mut warnings: Vec<(usize, String, String, (i32, i32))> = Vec::new();

        for animal in self.world.animals.get_all() {
            if !animal.is_alive() || animal.is_domesticated {
                continue;
            }

            let species = match self.world.animals.get_species(&animal.species_id) {
                Some(species) => species,
                None => continue,
            };

            // Somebody inside the ground of a beast that keeps one is
            // charged, hungry or not, whatever their age, and the beast takes
            // nothing for it.
            if Self::would_it_charge(species) {
                let too_near = agent_positions
                    .iter()
                    .filter(|(_, position)| {
                        (position.0 - animal.position.0)
                            .abs()
                            .max((position.1 - animal.position.1).abs())
                            <= Self::A_BEASTS_GROUND
                    })
                    .min_by_key(|(_, position)| {
                        (position.0 - animal.position.0).abs()
                            + (position.1 - animal.position.1).abs()
                    });
                if let Some((agent_index, _)) = too_near {
                    // It comes at them first and stops short. Somebody it has
                    // already warned off this half hour, and who is still
                    // there, it strikes. Striking at once, a mammoth charged
                    // sixty-two times in a year on seed 0, at forty a blow,
                    // and eight of twelve died of what the blows left them
                    // open to.
                    let warned = {
                        let emotions = &self.population.agents[*agent_index].emotions;
                        emotions.what_last_struck_me
                            == Some(crate::agents::EmotionSource::Creature(animal.species_id.clone()))
                            && current_turn.saturating_sub(emotions.last_attack_turn)
                                <= crate::environment::seasons::MINUTES_PER_TURN
                    };
                    if !warned {
                        warnings.push((
                            *agent_index,
                            animal.species_id.clone(),
                            species.name.clone(),
                            animal.position,
                        ));
                        continue;
                    }
                    if rng.gen::<f32>() < Self::HOW_OFTEN_IT_CHARGES {
                        strikes.push((
                            animal.id,
                            animal.species_id.clone(),
                            *agent_index,
                            species.attack_damage,
                            0.0,
                        ));
                        continue;
                    }
                }
            }

            if !animal.is_hungry() {
                continue;
            }

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

            // Nothing strikes at a grown person several times its own
            // weight. An eagle does not take a man; it struck six times in a
            // year on four seeds once people walked the country (ISSUES_FOUND
            // #274).
            if !Self::could_it_take(species, self.population.agents[*agent_index].state.years_old()) {
                continue;
            }

            // A full belly makes a cautious animal. Hunger is what changes
            // its mind, and only really at the end of it.
            let pressure =
                ((animal.hunger / animal.max_hunger.max(1.0)) - 0.5).clamp(0.0, 0.5) / 0.5;
            let odds = 0.01 + pressure * pressure * 0.14;

            if rng.gen::<f32>() < odds {
                strikes.push((
                    animal.id,
                    animal.species_id.clone(),
                    *agent_index,
                    species.attack_damage,
                    species.food_value * 0.25,
                ));
            }
        }

        // Warned off: frightened, deciding again now, and remembering the
        // ground as somewhere it went badly, which is what keeps them off it
        // tomorrow - the food that brought them there is weighed against it
        // (`how_bad_is_it_there`). No blow lands, and nothing is learned
        // about fighting from it. Without the memory a settlement beside a
        // mammoth was warned off 332 times in a year on seed 0 and walked
        // back each time.
        // By night or by day, for the measurement (#379)
        let (night, _, _) = self.where_in_the_night_it_is();
        if night {
            self.world.beasts_warned_by_night += warnings.len() as u64;
            self.world.beasts_struck_by_night += strikes.len() as u64;
        } else {
            self.world.beasts_warned_by_day += warnings.len() as u64;
            self.world.beasts_struck_by_day += strikes.len() as u64;
        }
        for (agent_index, what_it_was, called, at) in warnings {
            // And backs off it. Warned, a person decided again but was not
            // moved, so whatever they were about - a gather, a sleep, a site
            // going up - kept them inside the ground for the rest of the half
            // hour, and the beast is asked every minute: a quarter of a
            // chance a minute is a blow nearly every time. Each boar's dead
            // took four or five charges (#372).
            let here = self.population.agents[agent_index].state.position;
            let away = ((here.0 - at.0).signum(), (here.1 - at.1).signum());
            let away = if away == (0, 0) { (1, 0) } else { away };
            let off_its_ground = (Self::A_BEASTS_GROUND + 1..=Self::A_BEASTS_GROUND + 3)
                .map(|step| (at.0 + away.0 * step, at.1 + away.1 * step, here.2))
                .find(|to| {
                    self.world.is_terrain_passable(*to)
                        && self.world.can_walk_between((here.0, here.1), (to.0, to.1))
                });
            let agent = &mut self.population.agents[agent_index];
            if let Some(to) = off_its_ground {
                agent.state.position = to;
            }
            agent.busy_until = 0;
            agent.state.next_decision_at = agent.state.next_decision_at.min(current_turn);
            agent
                .emotions
                .record_attack(crate::agents::EmotionSource::Creature(what_it_was), current_turn);
            agent.exploration_knowledge.saw_danger(
                crate::world::Position::new(at.0, at.1),
                &called,
                1.0,
                current_turn,
            );
        }

        for (animal_id, what_it_was, agent_index, damage, fed) in strikes {
            // And a blow from one makes a stranger of the kind again
            if let Some(species) = self.world.animals.get_species(&what_it_was) {
                self.population.agents[agent_index].familiar_with.remove(&species.name);
            }
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

                // "Agents should not be peacefully sleeping while being
                // attacked." Whatever they were about, sleep included, is
                // broken off, and they decide again now.
                agent.busy_until = 0;
                agent.state.next_decision_at = agent.state.next_decision_at.min(current_turn);

                agent.emotions.record_attack(
                    crate::agents::EmotionSource::Creature(what_it_was.clone()),
                    current_turn,
                );

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
