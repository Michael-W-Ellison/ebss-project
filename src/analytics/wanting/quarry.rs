// src/analytics/wanting/quarry.rs
//! Hunting and fishing: going after something that would rather not be
//! caught.
//!
//! Part of the decision layer - see [`super`]. Nothing here does anything: it
//! answers what would be worth doing, and hands that answer back up the ladder.

use super::super::Simulation;
use crate::core::DriveType;
use crate::environment::Action;

impl Simulation {
    /// Whether this agent should be taking on this animal at all.
    ///
    /// Anything that fights back is a job for someone with a weapon in hand.
    /// An unarmed agent that walks up to a bear is not hunting, it is dying.
    pub(in crate::analytics) fn worth_hunting(
        &self,
        agent: &crate::agents::Agent,
        animal: &crate::environment::Animal,
    ) -> bool {
        if !animal.is_alive() || animal.is_domesticated || animal.is_kept() {
            return false;
        }

        let species = match self.world.animals.get_species(&animal.species_id) {
            Some(species) => species,
            None => return false,
        };

        // Nothing this one could not bring down with what it is carrying.
        // The executor has asked this since hunting was written and the
        // decision layer never did, so an agent walked to a deer it had no
        // means of killing and threw the turn away being refused. See
        // `Simulation::could_bring_it_down`.
        if !Self::could_bring_it_down(agent, species) {
            return false;
        }

        Self::fit_to_take_it_on(agent, species)
    }

    /// Whether a hunted beast of this kind turns on the hunter.
    ///
    /// The same question `hunting` asks when the throw misses: an aggressive,
    /// defensive or territorial beast with a blow worth the name.
    pub(in crate::analytics) fn fights_back(species: &crate::environment::fauna::AnimalSpecies) -> bool {
        use crate::environment::AnimalBehavior;
        matches!(
            species.behavior,
            AnimalBehavior::Aggressive | AnimalBehavior::Defensive | AnimalBehavior::Territorial
        ) && species.attack_damage > 0.0
    }

    /// Whether this one is in a state to start a fight with a beast of this
    /// kind: asked when the hunt is chosen and again when it is made, since a
    /// hunt can be chosen one half hour and made the next, after a blow.
    pub(in crate::analytics) fn fit_to_take_it_on(
        agent: &crate::agents::Agent,
        species: &crate::environment::fauna::AnimalSpecies,
    ) -> bool {
        use crate::environment::AnimalBehavior;

        if !Self::fights_back(species) {
            return true;
        }

        // Anything that comes after you is a job for someone with a weapon.
        let dangerous = matches!(
            species.behavior,
            AnimalBehavior::Aggressive | AnimalBehavior::Territorial
        );
        // A hunting weapon, carried: the old question asked the equipment
        // slot, which nothing fills, so nobody ever went after one (#100).
        let a_hunting_weapon = agent.equipment.get_weapon().is_some()
            || agent
                .what_i_have_to_work_with(crate::agents::skills::SkillType::Hunting)
                .is_some();
        if dangerous && !a_hunting_weapon {
            return false;
        }

        // And for a body that could take the blow it is inviting.
        //
        // The reactive half of this has always been asked:
        // `could_i_fight_at_all` is what decides whether a man stands his
        // ground when something comes at him, and `between_us::threat` asks it
        // of exactly this argument - `species.attack_damage`. The decision to
        // go and *start* the fight asked nothing at all about the man. Health,
        // a working arm, and whether he is old enough to fight were all absent
        // from `worth_hunting`, so **a man at ten health with a spear set off
        // after a bear exactly as a whole man would**, and a child did too.
        //
        // One question, asked in one place, on both sides of it: the same
        // predicate that lets him stand his ground decides whether he goes
        // looking for the fight. Ordinary prey is untouched - a rabbit is not
        // `dangerous` and a hurt man may still eat - so this declines the
        // fights that would kill him and none of the ones that would feed him.
        //
        // **And fit for it bare-handed.** A hunt is a spear thrown, and a
        // throw that misses can leave the spear in the bracken, so the beast
        // that turns on him may find nothing in his hands. Weighed against
        // the blow as the spear would turn it, a hunter with forty-odd health
        // went after a mammoth, lost his spear, and died of the whole blow -
        // on two worlds of sixteen (#327).
        //
        // Anything that turns on a hunter is weighed so, a cow as much as a
        // bear: a cow's blow is small, and it is still a blow (#327).
        agent.could_i_fight_at_all(species.attack_damage)
            && agent.state.health > species.attack_damage
    }

    /// How many of what this one wants come off one of these, as this one
    /// would take it apart, on a carcass as fat as the time of year makes it.
    ///
    /// `wanted` is asked of what each drop is carried as once cut - `meat`,
    /// `hides`, `wool` - so the same reckoning prices a hunt for supper and a
    /// hunt for a coat. An expectation, not a roll: the middle of each drop's
    /// range at its chance of coming off at all.
    pub(in crate::analytics) fn what_one_of_these_gives(
        &self,
        agent: &crate::agents::Agent,
        species: &crate::environment::fauna::AnimalSpecies,
        wanted: impl Fn(&str) -> bool,
    ) -> f32 {
        let condition = self.world.climate.how_fat_the_beasts_are();
        let knife = agent.how_much_my_tools_bring_back(crate::agents::skills::SkillType::Leatherworking);

        species
            .drops
            .iter()
            .filter(|drop| {
                wanted(crate::agents::storage_integration::butchered_item_id(&drop.material_id))
            })
            .map(|drop| {
                let middle = (drop.min_quantity + drop.max_quantity) as f32 / 2.0;
                middle * drop.drop_chance * condition * knife
            })
            .sum()
    }

    /// How many turns of throwing it takes this one to bring down an animal
    /// with this much life left in it.
    ///
    /// The throws that have to tell, over how often one does - both as the
    /// executor will have them, from `what_a_throw_takes_off` and
    /// `how_often_a_throw_tells`. A flint spear in a grown arm is two throws
    /// that tell for a deer and something like eight turns of throwing for
    /// somebody who has never hunted; a stone at a rabbit is three that tell.
    pub(in crate::analytics) fn turns_to_bring_it_down(
        agent: &crate::agents::Agent,
        life_left: f32,
    ) -> f32 {
        let point = agent.how_fast_my_tools_make_this_go(crate::agents::skills::SkillType::Hunting);
        let one_throw = Self::what_a_throw_takes_off(agent.own_strength(), point)
            * (1.0 + agent.transport.mounted_combat_bonus());
        let that_tell = (life_left / one_throw.max(0.1)).ceil().max(1.0);

        that_tell / Self::how_often_a_throw_tells(agent, agent.equipment.get_weapon().is_some())
    }

    /// What share of a hunt is left once the blows it will cost are paid for.
    ///
    /// Every throw that misses something that fights back is a blow taken
    /// (`hunting`), so a hunt of `turns` throws at a beast that turns is that
    /// many misses' worth of its blow, as what is in the hand turns it. A hunt
    /// that would cost half of what this one has left is worth half as much to
    /// him, and one that would cost all of it is worth nothing - which is the
    /// same question `fit_to_take_it_on` asks of a single blow, asked of the
    /// whole hunt.
    pub(in crate::analytics) fn what_is_left_after_the_blows(
        agent: &crate::agents::Agent,
        species: &crate::environment::fauna::AnimalSpecies,
        turns: f32,
    ) -> f32 {
        if !Self::fights_back(species) {
            return 1.0;
        }
        let tells = Self::how_often_a_throw_tells(agent, agent.equipment.get_weapon().is_some());
        let misses = turns * (1.0 - tells);
        let hurt = misses * agent.what_a_blow_costs_me(species.attack_damage);

        (1.0 - hurt / agent.state.health.max(1.0)).max(0.0)
    }

    /// What going after this animal for its meat is worth, in the same coin a
    /// forage patch is: food brought home less what the trip costs, over the
    /// turns it takes - see `what_this_patch_is_worth`.
    ///
    /// "The yield of the hunt should be considered. A small bird results in
    /// little meat, whereas a deer results in significantly more... effort,
    /// time, and results should all be considered." Prey was the *nearest*
    /// animal there was any means of killing, and nothing asked what was on
    /// it: a hungry man threw at a kestrel, which has no meat on it at all,
    /// as readily as at a deer, and six of the eleven kills over twelve
    /// settlement-years were kestrels (#330).
    ///
    /// **Results**: the meat on it, as much of it as there is room for in the
    /// pack, at what the portions it cuts into are worth to eat. A joint is
    /// three portions (`CUT_MEAT_INTO_PORTIONS`), and it is the portion that
    /// is eaten.
    ///
    /// **Time**: the turns of throwing it takes as this hunter throws, the
    /// walk there and back, and a turn to take it apart.
    ///
    /// **Effort**: the walk and every throw, landed or not.
    ///
    /// And **what it costs him**: the blows a beast that fights back will
    /// land on the throws that miss.
    pub(in crate::analytics) fn what_this_hunt_is_worth_to_eat(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
        animal: &crate::environment::Animal,
        species: &crate::environment::fauna::AnimalSpecies,
    ) -> f32 {
        let joints = self.what_one_of_these_gives(agent, species, |cut| cut == "meat");
        if joints <= 0.0 {
            return 0.0;
        }

        // What will go in the pack: the rest stays where it fell.
        let room_for = agent.inventory.weight_capacity_remaining() / Self::WHAT_A_JOINT_WEIGHS;
        let portions = joints.min(room_for)
            * crate::environment::making::CUT_MEAT_INTO_PORTIONS.how_many as f32;

        let energy = self
            .food_database
            .get(&crate::world::ItemType::Meat)
            .map(|template| template.base_nutrition.energy)
            .unwrap_or(crate::agents::physiology::ENERGY_OF_ORDINARY_FOOD);
        let brings_back = portions
            * crate::agents::physiology::UNITS_IN_ONE_ITEM
            * crate::agents::physiology::what_a_unit_of_this_is_worth(energy);

        let paces = ((animal.position.0 - agent_position.0).abs()
            + (animal.position.1 - agent_position.1).abs()) as u32;
        let throwing = Self::turns_to_bring_it_down(agent, animal.current_health);
        let costs = crate::agents::provision::what_foraging_costs(
            paces,
            crate::agents::physiology::how_much_work_this_food_is(energy),
        ) + throwing * Self::WHAT_A_THROW_COSTS;

        // The throwing, a turn with the knife, and the walk there and back.
        let turns = throwing
            + 1.0
            + crate::world::pace::turns_to_walk(paces) * Self::TURNS_A_PACE_TAKES;

        (brings_back - costs) / turns * Self::what_is_left_after_the_blows(agent, species, throwing)
    }

    /// The animal within `reach` most worth going after for its meat, where it
    /// is, and what it is worth - `None` when nothing in reach is worth a
    /// throw for supper.
    pub(in crate::analytics) fn the_best_prey_to_eat(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
        reach: i32,
    ) -> Option<(uuid::Uuid, (i32, i32), f32)> {
        self.world
            .get_animals_in_radius(
                (agent_position.0, agent_position.1),
                Self::how_far_a_hunter_looks(agent),
            )
            .into_iter()
            .filter(|animal| {
                (animal.position.0 - agent_position.0)
                    .abs()
                    .max((animal.position.1 - agent_position.1).abs())
                    <= reach
            })
            .filter(|animal| self.worth_hunting(agent, animal))
            .filter_map(|animal| {
                let species = self.world.animals.get_species(&animal.species_id)?;
                let worth = self.what_this_hunt_is_worth_to_eat(agent, agent_position, animal, species);
                (worth > 0.0).then_some((animal.id, animal.position, worth))
            })
            .max_by(|(_, _, a), (_, _, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
    }

    /// The animal most worth going after for its skin, and where it is.
    ///
    /// What a coat is cut from - hides, leather, wool - per turn the hunt
    /// takes, less the blows it costs. It was the nearest animal there was a
    /// means of killing, which sent a man wanting a fur coat after a kestrel
    /// for its feathers (#330).
    pub(in crate::analytics) fn the_best_prey_for_its_skin(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> Option<(uuid::Uuid, (i32, i32))> {
        self.world
            .get_animals_in_radius(
                (agent_position.0, agent_position.1),
                Self::how_far_a_hunter_looks(agent),
            )
            .into_iter()
            .filter(|animal| self.worth_hunting(agent, animal))
            .filter_map(|animal| {
                let species = self.world.animals.get_species(&animal.species_id)?;
                let skins = self.what_one_of_these_gives(agent, species, |cut| {
                    matches!(cut, "hides" | "leather" | "wool")
                });
                if skins <= 0.0 {
                    return None;
                }
                let paces = ((animal.position.0 - agent_position.0).abs()
                    + (animal.position.1 - agent_position.1).abs()) as u32;
                let throwing = Self::turns_to_bring_it_down(agent, animal.current_health);
                let turns = throwing
                    + 1.0
                    + crate::world::pace::turns_to_walk(paces) * Self::TURNS_A_PACE_TAKES;
                let worth = skins / turns * Self::what_is_left_after_the_blows(agent, species, throwing);
                (worth > 0.0).then_some((animal.id, animal.position, worth))
            })
            .max_by(|(_, _, a), (_, _, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(id, position, _)| (id, position))
    }

    /// Whether the agent has a reason to go after an animal.
    ///
    /// Two of them: nothing to eat, or nothing warm to wear and no skins to
    /// make it from. Fur and hides are the warm half of the garment table and
    /// the only way to them is off an animal.
    pub(in crate::analytics) fn wants_to_hunt(agent: &crate::agents::Agent) -> bool {
        // An agent hunts for skins, and the meat is a bonus.
        //
        // Hunting for the meat as such does not pay: berries and fish are
        // there for the taking and an animal has to be found, walked to and
        // hit. Agents that went after every animal because their pack was
        // empty starved for it, and two settlements in forty died out.
        //
        // It also keeps hunting until there are enough skins for the garment,
        // not until there is one skin: a fur coat takes five hides, and an
        // agent that stopped at the first came home with a single pelt over
        // and over and never wore anything warmer than woven flax.
        if !Self::wants_more_clothing(agent) {
            return false;
        }

        let quality = Self::expected_garment_quality(agent);

        let wants = crate::agents::equipment::GARMENT_RECIPES.iter().any(|recipe| {
            matches!(recipe.material_item, "hides" | "leather" | "wool")
                && agent.knows_how_to_make_garment(recipe.id)
                && Self::worth_making(
                    Self::garment_warmth(recipe, quality),
                    Self::warmth_worn(agent, recipe.slot),
                )
        });

        if !wants {
            return false;
        }

        // Stop once there is enough of anything to make one. An agent with a
        // pack full of hides has no business going after a sheep for the wool
        // it has never had.
        let can_already_make = crate::agents::equipment::GARMENT_RECIPES.iter().any(|recipe| {
            matches!(recipe.material_item, "hides" | "leather" | "wool")
                && agent.knows_how_to_make_garment(recipe.id)
                && Self::worth_making(
                    Self::garment_warmth(recipe, quality),
                    Self::warmth_worn(agent, recipe.slot),
                )
                && Self::can_spare_material(agent, recipe)
        });

        !can_already_make
    }

    /// Going after an animal: strike if it is within reach, close on it if not.
    ///
    /// Nothing in the simulation had ever selected `Action::Hunt` - the one
    /// place it appeared passed a nil animal id that the executor could not
    /// resolve - so no agent had ever hunted, and meat, hides and wool never
    /// reached an inventory at all.
    pub(in crate::analytics) fn hunting_action(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> Option<Action> {
        if Self::wants_to_hunt(agent) {
            if let Some(going) = self.going_after_skins(agent, agent_position) {
                return Some(going);
            }
        }
        if agent.craves_meat(self.current_turn) {
            return self.going_after_meat(agent, agent_position);
        }
        None
    }

    /// How far a hunter looks for game: as far as they can see (#358).
    ///
    /// It was twelve cells, a hundred and twenty metres, set when a beast
    /// moved two cells a turn. At twenty-five times that pace (`world::pace`)
    /// a deer grazes through a hundred and twenty metres in a quarter hour,
    /// and a hunter saw almost nothing worth a throw: measured on two worlds,
    /// four in five hunts wanted found no prey.
    pub(in crate::analytics) fn how_far_a_hunter_looks(agent: &crate::agents::Agent) -> f32 {
        Self::HUNT_SEARCH_RADIUS.max(agent.sight_range() as f32)
    }

    /// How far a hunter walks to where game was lately seen.
    pub(in crate::analytics) const HOW_FAR_A_HUNTER_GOES: u32 = 150;

    /// Going after an animal for its meat (#358), by somebody who craves
    /// it: one worth eating in sight, struck or closed on; else a walk to
    /// where game was lately seen.
    pub(in crate::analytics) fn going_after_meat(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> Option<Action> {
        use crate::agents::practices::Undertaking;
        if !agent.lessons.worth_trying(Undertaking::Hunting) {
            return None;
        }
        if let Some(getting_one) = self.what_a_hunt_wants_first(agent) {
            return Some(getting_one);
        }
        let sight = Self::how_far_a_hunter_looks(agent) as i32;
        if let Some((animal_id, animal_position, _)) = self.the_best_prey_to_eat(agent, agent_position, sight) {
            let reach = (animal_position.0 - agent_position.0)
                .abs()
                .max((animal_position.1 - agent_position.1).abs());
            if reach <= Self::HUNT_REACH {
                return Some(Action::Hunt {
                    animal_id,
                    weapon: agent.equipment.get_weapon().map(|weapon| weapon.name.clone()),
                });
            }
            return Some(Action::Move { target: (animal_position.0, animal_position.1, agent_position.2) });
        }
        let now = self.current_turn;
        let here = (agent_position.0, agent_position.1);
        agent
            .game_seen
            .iter()
            .filter(|(_, when)| now.saturating_sub(*when) < crate::agents::Agent::GAME_STAYS_ABOUT)
            .filter(|(there, _)| {
                let far = (there.0 - here.0).abs().max((there.1 - here.1).abs());
                far > sight && far as u32 <= Self::HOW_FAR_A_HUNTER_GOES
            })
            .filter(|(there, _)| self.world.can_walk_between(here, *there))
            .max_by_key(|(there, when)| (*when, there.0, there.1))
            .map(|(there, _)| Action::Move { target: (there.0, there.1, agent_position.2) })
    }

    /// Going after an animal for its skin, for whatever reason the skin is
    /// wanted: a garment (`hunting_action`) or a tent's covering
    /// (`raising_a_roof`, #352).
    pub(in crate::analytics) fn going_after_skins(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> Option<Action> {
        use crate::agents::practices::Undertaking;

        // Somebody who has gone after animals a dozen times and come back
        // empty every time stops going after animals. Nothing tells them to:
        // it is what their own record says, and a hunter with a good record
        // keeps at it on the same evidence.
        if !agent.lessons.worth_trying(Undertaking::Hunting) {
            return None;
        }

        // Nothing to throw: the spear is the job.
        //
        // This is the preparation the model did not have. Wanting to hunt was
        // decided here, the walk to the animal was taken, and only when the
        // agent was standing over it did anything ask whether it had a spear -
        // at which point `make_what_this_wants` went looking for the makings
        // of one in whatever wood or meadow the deer happened to be standing
        // in, and did not find them. Measured over six worlds: 643 hunts
        // reached that question, 633 of them wanted a spear, and in 613 no
        // step in the chain could be taken from where the agent was.
        //
        // Wanting a thing you have not got is a reason to go and get it, and
        // the place to do that from is wherever the agent is when it forms the
        // want - beside the camp, where the stone and the wood are - not
        // beside the animal. So the answer to "I would hunt" from somebody
        // empty-handed is the next step towards a spear, and the errand
        // machinery carries it the rest of the way.
        if let Some(getting_one) = self.what_a_hunt_wants_first(agent) {
            return Some(getting_one);
        }

        let (animal_id, animal_position) = self.the_best_prey_for_its_skin(agent, agent_position)?;

        let reach = (animal_position.0 - agent_position.0)
            .abs()
            .max((animal_position.1 - agent_position.1).abs());

        if reach <= Self::HUNT_REACH {
            return Some(Action::Hunt {
                animal_id,
                weapon: agent.equipment.get_weapon().map(|weapon| weapon.name.clone()),
            });
        }

        Some(Action::Move {
            target: (animal_position.0, animal_position.1, agent_position.2),
        })
    }

    /// Getting hold of something to hunt with, for somebody who means to hunt
    /// and has nothing.
    ///
    /// `None` when there is nothing to be done about it here - which is the
    /// honest answer for a man in a meadow with no stone in it, and leaves the
    /// hunt to be attempted with bare hands against whatever a thrown stone
    /// will still bring down.
    pub(in crate::analytics) fn what_a_hunt_wants_first(
        &self,
        agent: &crate::agents::Agent,
    ) -> Option<Action> {
        self.what_this_trade_wants_first(agent, crate::agents::skills::SkillType::Hunting)
    }

    /// The next step towards the humblest tool of a trade this one has none
    /// of, or `None` when they have one already - or no step can be taken.
    pub(in crate::analytics) fn what_this_trade_wants_first(
        &self,
        agent: &crate::agents::Agent,
        trade: crate::agents::skills::SkillType,
    ) -> Option<Action> {
        if agent.what_i_have_to_work_with(trade).is_some() {
            return None;
        }

        // The humblest thing that will do, not the best thing known.
        //
        // `what_i_would_rather_have` answers a different question - it is the
        // *upgrade*, and takes the highest `how_much_better` the agent knows
        // how to make. Asked by somebody with nothing at all it names the bow,
        // and a man who cannot come by a bow this afternoon then does nothing,
        // when a sharpened stick was three turns away. Measured with the best
        // one asked for: 1,881 wants of a hunting tool in six worlds and 340
        // that came to anything.
        //
        // So the ladder is walked from the bottom and the first rung this one
        // can actually get onto is the answer. Getting onto it is what makes
        // the next rung reachable later; `would_a_better_tool_pay` is what
        // climbs.
        let mut ladder: Vec<&'static crate::environment::making::Tool> =
            crate::environment::making::what_helps_with(trade)
                .filter(|tool| agent.knows_how_to_make(tool.called))
                .collect();
        ladder.sort_by(|a, b| {
            a.how_much_better
                .partial_cmp(&b.how_much_better)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        ladder
            .into_iter()
            .find_map(|tool| self.how_i_would_come_by(tool.called, agent))
    }

    /// How far a parent lets a child of its own get before going after it
    pub(in crate::analytics) const CHILD_LEASH: i32 = 8;

    /// How far an agent can work a reach from where it stands
    pub(in crate::analytics) const CAST: i32 = 1;

    /// How far an agent will walk to get to water.
    ///
    /// A hundred and forty metres was seven hours' walk at a cell a turn. At
    /// five kilometres an hour, ten times that is seventeen minutes.
    pub(in crate::analytics) const WORTH_WALKING_TO_WATER: i32 = 14 * crate::world::pace::HOW_MUCH_FURTHER_IS_WORTH_WALKING as i32;

    /// A reach carrying this many fish is as good as fishing gets
    pub(in crate::analytics) const A_GOOD_REACH: f32 = 60.0;

    /// What comes out of the water on a cast that works
    pub(in crate::analytics) const FISH_PER_CAST: u32 = 2;

    /// How often a thrust tells in an empty reach, for somebody with nothing
    /// in his hands.
    ///
    /// Everything worth having is added to this: the thickness of the run, the
    /// hand, a rod, a spear. On its own it is a man standing in a river
    /// hoping.
    pub(in crate::analytics) const A_THRUST_THAT_TELLS: f32 = 0.15;

    /// What standing in the water costs, whether or not anything takes.
    pub(in crate::analytics) const WHAT_A_THRUST_COSTS: f32 = 8.0;

    /// What share of a fish is guts, heads and bone rather than meat.
    ///
    /// It goes to waste in the pack the moment the fish is caught, which is
    /// what puts a fishing agent in the way of doing a field good without ever
    /// meaning to.
    pub(in crate::analytics) const OFFAL_SHARE: f32 = 0.35;

    /// The chance a thrust or a cast comes to something, for a hand of this
    /// skill with tackle this good (`how_fast_my_tools_make_this_go`) in a
    /// reach this thick, nought to one.
    pub(in crate::analytics) fn odds_of_a_cast(skill: f32, tackle: f32, thickness: f32) -> f32 {
        let hand = (skill / 10.0).clamp(0.0, 0.5) + (tackle - 1.0) * 0.3;
        (Self::A_THRUST_THAT_TELLS + 0.4 * thickness + hand).clamp(0.0, 0.9)
    }

    /// And the chance a turn picking over the shallows of a reach this thick
    /// comes to a fish, a handful of shellfish or a crab.
    ///
    /// Steadier than a thrust and smaller: no skill and no tackle come into
    /// it, and what the edge of a thin reach holds is still there to be
    /// turned over when the fish have gone deep.
    pub(in crate::analytics) fn odds_in_the_shallows(thickness: f32) -> f32 {
        Self::WHAT_THE_SHALLOWS_GIVE_A_HAND
            * (Self::WHAT_THIN_SHALLOWS_GIVE + (1.0 - Self::WHAT_THIN_SHALLOWS_GIVE) * thickness)
    }

    /// How often a turn picking over a full reach's shallows comes to
    /// something.
    pub(in crate::analytics) const WHAT_THE_SHALLOWS_GIVE_A_HAND: f32 = 0.8;

    /// And what share of that the shallows of an empty-looking reach still
    /// give.
    pub(in crate::analytics) const WHAT_THIN_SHALLOWS_GIVE: f32 = 0.6;

    /// The reach an agent standing here can work, if there is one.
    pub(in crate::analytics) fn reach_within_cast(
        &self,
        agent_position: (i32, i32, i32),
    ) -> Option<crate::world::Position> {
        self.world
            .nodes_near(
                crate::world::Position::new(agent_position.0, agent_position.1),
                Self::CAST as u32,
            )
            .filter(|resource| resource.resource_type.grows_in_water())
            .filter(|resource| resource.amount > 0)
            .map(|resource| resource.position)
            .find(|position| {
                (position.x - agent_position.0).abs() <= Self::CAST
                    && (position.y - agent_position.1).abs() <= Self::CAST
            })
    }

    /// Standing in a river after fish, and walking to a river worth standing in.
    ///
    /// A fishery is not another way of getting a meal. It is the only food a
    /// settlement can take that the land does not pay for, because a fish is
    /// grown at sea and comes up the river under its own power - so what is
    /// left of it, put on a field, makes the country richer rather than
    /// slower to run down. Everything else a settlement does with the ground
    /// is at best a return of what it already took.
    ///
    /// Nobody is told this. An agent fishes because it is hungry and there is
    /// water; the guts go into its pack as waste like anything else; and if it
    /// has learned that tipping the pack on a field does the ground good, the
    /// two habits meet on their own. What the agent keeps of that meeting is
    /// its own record of whether fishing pays - a person who stood in an empty
    /// winter river a dozen times stops going.
    pub(in crate::analytics) fn fishing_action(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> Option<Action> {
        // Fishing is what an agent does when it wants food or wants a store of
        // it. Both, in a settlement beside a river, most of the year.
        let hunger = agent
            .drives
            .get(DriveType::Hunger)
            .map(|drive| drive.urgency())
            .unwrap_or(0.0);
        let sustenance = agent
            .drives
            .get(DriveType::Sustenance)
            .map(|drive| drive.urgency())
            .unwrap_or(0.0);

        if hunger.max(sustenance) < Self::WORTH_GETTING_WET {
            return None;
        }

        // And nobody stands in a river for more fish than he will eat. This is
        // where it bites hardest: whole fish is the largest single thing that
        // goes off in anybody's pack in this model.
        if Self::more_food_than_he_will_get_through(agent) {
            return None;
        }

        self.going_fishing(agent, agent_position)
    }

    /// To the water: his own traps, somebody else's, a cast from where he
    /// stands, or a walk to the best water about.
    ///
    /// What `fishing_action` does once hunger has said to go, and what
    /// putting food by does when the store is short (`going_out_for_food_to_
    /// put_by`): the river does not care which of them sent him.
    pub(in crate::analytics) fn going_fishing(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> Option<Action> {
        use crate::agents::practices::Undertaking;

        // Fish in his own traps come first, whatever he has come to believe
        // about standing in a river: a catch is a catch.
        if let Some(going) = self.fish_in_my_traps(agent, agent_position) {
            return Some(going);
        }

        // And somebody else's, if nobody is looking and it is worth it.
        if let Some(whose) = self.a_trap_worth_robbing(agent, agent_position) {
            return Some(Action::RobFishTrap { whose });
        }

        // Somebody who has stood in the water a dozen times and come out with
        // nothing stops going to the water.
        if !agent.lessons.worth_trying(Undertaking::Fishing) {
            return None;
        }

        if let Some(reach) = self.reach_within_cast(agent_position) {
            return Some(self.how_to_work_this_water(agent, reach));
        }

        // Otherwise walk to the best water within reason: the thickest reach,
        // discounted by how far it is. A river in the run is worth crossing a
        // settlement for and an empty pool next door is not.
        let (best, _) = self
            .nodes_known_to(
                agent,
                crate::world::Position::new(agent_position.0, agent_position.1),
                Self::WORTH_WALKING_TO_WATER as u32,
            )
            .filter(|resource| resource.resource_type.grows_in_water())
            .filter(|resource| resource.amount > 0)
            .filter_map(|resource| {
                let reach = (resource.position.x - agent_position.0)
                    .abs()
                    .max((resource.position.y - agent_position.1).abs());

                if reach > Self::WORTH_WALKING_TO_WATER {
                    return None;
                }

                let worth = resource.amount as f32 / (1.0 + reach as f32);
                Some((resource.position, worth))
            })
            .fold(
                None,
                |best: Option<(crate::world::Position, f32)>, (position, worth)| {
                    match best {
                        Some((_, best_worth)) if best_worth >= worth => best,
                        _ => Some((position, worth)),
                    }
                },
            )?;

        Some(Action::Move {
            target: (best.x, best.y, agent_position.2),
        })
    }

    /// A trap of this one's with fish in it: empty it if it is within a cast,
    /// walk to it if it is within a line's walk.
    ///
    /// The same bound a trapline keeps (`AS_FAR_AS_A_LINE_IS_WORTH_WALKING`):
    /// traps are set round where a people is, and one left behind when the
    /// camp moved on is the river's again.
    pub(in crate::analytics) fn fish_in_my_traps(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> Option<Action> {
        let here = (agent_position.0, agent_position.1);
        let paces = |trap: &crate::world::FishTrap| (trap.at.x - here.0).abs().max((trap.at.y - here.1).abs());
        let nearest = self
            .world
            .fish_traps
            .iter()
            .filter(|trap| trap.set_by == agent.id && trap.holds > 0)
            .filter(|trap| paces(trap) <= Self::AS_FAR_AS_A_LINE_IS_WORTH_WALKING)
            .min_by_key(|trap| paces(trap))?;

        if paces(nearest) <= Self::CLOSE_ENOUGH_TO_A_SNARE {
            Some(Action::CheckFishTraps)
        } else {
            Some(Action::Move {
                target: (nearest.at.x, nearest.at.y, agent_position.2),
            })
        }
    }

    /// Somebody else's trap within reach with fish in it, that this agent
    /// would empty: weighed the way taking from somebody's pack is weighed -
    /// what it would answer against who would see it (`would_i_take_it`).
    ///
    /// A trap is an easier thing to rob than a man, because its owner is
    /// seldom standing beside it. Never a trap of family or a partner's: that
    /// is not theft in a band, it is supper.
    pub(in crate::analytics) fn a_trap_worth_robbing(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> Option<uuid::Uuid> {
        use crate::agents::RelationshipType;

        let here = (agent_position.0, agent_position.1);
        let trap = self
            .world
            .fish_traps
            .iter()
            .filter(|trap| trap.set_by != agent.id && trap.holds > 0)
            .filter(|trap| {
                (trap.at.x - here.0).abs().max((trap.at.y - here.1).abs()) <= Self::CLOSE_ENOUGH_TO_A_SNARE
            })
            .filter(|trap| {
                !agent.relationships.get_relationship(&trap.set_by).is_some_and(|bond| {
                    bond.is_family() || bond.relationship_type == RelationshipType::Partner
                })
            })
            .max_by_key(|trap| trap.holds)?;

        // Whoever is looking, and the owner, who is not but will know: the
        // trap remembers who emptied it (`going_round_the_traps`).
        let what_the_owner_is_to_me = agent
            .relationships
            .get_relationship(&trap.set_by)
            .map_or(0.0, |bond| bond.bond_strength);
        let cost = self
            .what_taking_here_would_cost(agent, agent_position)
            .max(agent.what_taking_it_would_cost_me(1, what_the_owner_is_to_me));
        let gain = agent.what_taking_this_would_answer("fish", trap.holds);
        agent.would_i_take_it(gain, cost).then_some(trap.set_by)
    }

    /// What to do with a reach within a cast.
    ///
    /// Set a trap in it if he has one with him and it is not already trapped;
    /// make one if he knows how and has the makings, while he keeps fewer
    /// than he can; and otherwise whichever of a thrust and picking over the
    /// shallows he expects to bring more out of it. For somebody with no
    /// tackle and no hand for it, on a thin reach, that is the shallows.
    pub(in crate::analytics) fn how_to_work_this_water(
        &self,
        agent: &crate::agents::Agent,
        reach: crate::world::Position,
    ) -> Action {
        use crate::agents::SkillType;

        let traps_kept = self.world.fish_traps.iter().filter(|trap| trap.set_by == agent.id).count();
        let room_for_one = traps_kept < Self::TRAPS_IN_THE_WATER
            && !self.world.fish_traps.iter().any(|trap| trap.at == reach);
        if room_for_one && agent.knows_how_to_do("SetFishTrap") {
            if agent.how_many_i_have("fishtrap") > 0 {
                return Action::SetFishTrap;
            }
            let trap = &crate::environment::making::FISH_TRAP;
            if agent.knows_how_to(trap) && trap.makings_to_hand(|what| agent.how_many_i_have(what)) {
                return Action::Craft {
                    item_type: trap.makes.to_string(),
                };
            }
        }

        let standing = self.world.nodes_on(reach).next().map(|r| r.amount).unwrap_or(0);
        let thickness = (standing as f32 / Self::A_GOOD_REACH).clamp(0.0, 1.0);
        let skill = agent
            .skills
            .get_skill_if_exists(SkillType::Fishing)
            .map(|skill| skill.level)
            .unwrap_or(-10) as f32;
        let tackle = agent.how_fast_my_tools_make_this_go(SkillType::Fishing);
        let a_cast = Self::odds_of_a_cast(skill, tackle, thickness)
            * ((Self::FISH_PER_CAST as f32 * tackle).round()).max(1.0);
        let by_hand = Self::odds_in_the_shallows(thickness);

        if by_hand > a_cast && agent.knows_how_to_do("ForageShallows") {
            Action::ForageShallows
        } else {
            Action::Fish
        }
    }

    /// Traps in the water: make one, carry it to a reach nobody has trapped,
    /// and set it.
    ///
    /// Beside the snares in the Preparedness ladder, and for the same reason:
    /// a trap is food put by that keeps itself alive until it is wanted. It
    /// has to be asked for here rather than only at the waterside, because
    /// measured that way nobody ever made one - twelve people in sixteen knew
    /// how inside a fortnight, and in two years nobody was once standing at a
    /// reach with two wood and two lashing in their pack.
    pub(in crate::analytics) fn putting_traps_in_the_water(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> Option<Action> {
        if !agent.knows_how_to_do("SetFishTrap") {
            return None;
        }
        let hunger = agent.drives.get(DriveType::Hunger).map(|drive| drive.urgency()).unwrap_or(0.0);
        let putting_by = agent
            .drives
            .get(DriveType::Preparedness)
            .map(|drive| drive.urgency())
            .unwrap_or(0.0);
        if hunger.max(putting_by) < Self::WORTH_SETTING_STRING {
            return None;
        }
        let kept = self.world.fish_traps.iter().filter(|trap| trap.set_by == agent.id).count();
        let carried = agent.how_many_i_have("fishtrap") as usize;
        if kept + carried.min(1) > Self::TRAPS_IN_THE_WATER {
            return None;
        }
        let untrapped = |at: crate::world::Position| !self.world.fish_traps.iter().any(|trap| trap.at == at);

        // The nearest reach worth a trap that nobody has trapped, within the
        // walk a man would make to fish.
        let here = crate::world::Position::new(agent_position.0, agent_position.1);
        let paces = |at: crate::world::Position| (at.x - here.x).abs().max((at.y - here.y).abs());
        let water = self
            .nodes_known_to(agent, here, Self::WORTH_WALKING_TO_WATER as u32)
            .filter(|resource| resource.resource_type.grows_in_water())
            .filter(|resource| resource.amount > 0)
            .map(|resource| resource.position)
            .filter(|at| untrapped(*at) && paces(*at) <= Self::WORTH_WALKING_TO_WATER)
            .min_by_key(|at| paces(*at))?;

        if carried == 0 {
            if kept >= Self::TRAPS_IN_THE_WATER {
                return None;
            }
            let holding = |what: &str| agent.how_many_i_have(what);
            let knows = |step: &crate::environment::making::Making| agent.knows_how_to(step);
            let in_hand = |what: &str| agent.how_many_i_have(what) > 0;
            return crate::environment::making::what_to_do_first_that_can_be_done(
                crate::environment::making::FISH_TRAP.makes,
                &holding,
                &knows,
                &in_hand,
                false,
            )
            .map(|step| Action::Craft {
                item_type: step.makes.to_string(),
            });
        }

        match self.reach_within_cast(agent_position) {
            Some(reach) if untrapped(reach) => Some(Action::SetFishTrap),
            _ => Some(Action::Move {
                target: (water.x, water.y, agent_position.2),
            }),
        }
    }

    /// How much an agent has to want food before it will go and stand in a river
    pub(in crate::analytics) const WORTH_GETTING_WET: f32 = 0.35;

    /// How pressing a want has to be before it is worth setting string.
    ///
    /// Lower than the bar for wading into a river, because a snare is set and
    /// left: the cost is a turn now and the return comes days later, so it is
    /// the cheapest thing a hungry man can do about tomorrow.
    pub(in crate::analytics) const WORTH_SETTING_STRING: f32 = 0.25;

    /// How far an agent will go for a catch it knows is waiting.
    ///
    /// A kilometre and a half at ten metres a cell, which is eighteen
    /// minutes' walk. A trapline is set round where you live; anything
    /// further off is a snare you will come across when you come across it.
    /// It was a hundred and fifty metres while a walk was a cell a turn.
    pub(in crate::analytics) const AS_FAR_AS_A_LINE_IS_WORTH_WALKING: i32 = 15 * crate::world::pace::HOW_MUCH_FURTHER_IS_WORTH_WALKING as i32;

    /// A catch in a snare the agent is standing on. Free: no walk, no
    /// weighing, take it.
    ///
    /// This is the half that goes in front of everything else in the hunger
    /// chain, and it is safe there precisely because it costs nothing. The
    /// walk is a different question and is answered further down.
    pub(in crate::analytics) fn a_catch_at_my_feet(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> Option<Action> {
        let here = (agent_position.0, agent_position.1);

        self.world
            .snares
            .iter()
            .find(|snare| {
                snare.set_by == agent.id
                    && snare.is_holding_something()
                    && (snare.at.0 - here.0).abs().max((snare.at.1 - here.1).abs())
                        <= Self::CLOSE_ENOUGH_TO_A_SNARE
            })
            .map(|_| Action::CheckSnares)
    }

    /// A catch further off, and whether it is worth the walk.
    ///
    /// **This is the half that has to sit behind ordinary food, and it was
    /// measured the wrong way round first.** In front of `food_action` a
    /// hungry man walks across the country for one rabbit instead of eating
    /// the berry at his feet, and six worlds over a year went from 23,733
    /// person-days to 14,920 with thirty alive at the end against eleven.
    /// It is the same defect the larder branch carries a note about: a want
    /// that is real is not thereby the most pressing thing an agent can do
    /// with the turn.
    ///
    /// So: only what is within a short walk, and only after the ground in
    /// front of him has been asked.
    pub(in crate::analytics) fn walking_to_a_catch(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> Option<Action> {
        let here = (agent_position.0, agent_position.1);
        // A catch left where it is gets taken, and the thinner the country
        // the faster - which is why this is worth a turn at all. But only
        // what is within a walk: a line is something you keep near where you
        // live, and one snare the far side of the map is not a reason to
        // cross it.
        let waiting = self
            .world
            .snares
            .iter()
            .filter(|snare| snare.set_by == agent.id && snare.is_holding_something())
            .filter(|snare| {
                (snare.at.0 - here.0).abs().max((snare.at.1 - here.1).abs())
                    <= Self::AS_FAR_AS_A_LINE_IS_WORTH_WALKING
            })
            .min_by_key(|snare| {
                (snare.at.0 - here.0).abs() + (snare.at.1 - here.1).abs()
            });

        if let Some(snare) = waiting {
            return Some(Action::Move {
                target: (snare.at.0, snare.at.1, agent_position.2),
            });
        }

        None
    }

    /// The round: go and walk the line because it is time to, whether or not
    /// anything is known to be in it.
    ///
    /// **This is the piece that made a trapline a trapline.**
    /// `a_catch_at_my_feet` and `walking_to_a_catch` both ask whether a snare
    /// is *holding something* - which is to say the agent only ever went to a
    /// snare it already knew had a rabbit in it, and only once hunger had been
    /// refused by everything above them. A man does not know his snare has
    /// caught anything until he walks out and looks, and the walking out is
    /// the whole of the work.
    ///
    /// How often he goes is not written here. It is
    /// [`crate::agents::rhythm::Rhythm`], which he moves about by what the
    /// rounds bring back and which settles near the span the country takes to
    /// rob a snare - a number that lives in the ecology, not in this module.
    pub(in crate::analytics) fn going_round_is_due(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> Option<Action> {
        use crate::agents::practices::Undertaking;

        if !agent.is_it_time_i(Undertaking::Trapping, self.current_turn) {
            return None;
        }

        // Somebody who has worked out that trapping does not feed him keeps no
        // rhythm for it. This is the one gate the rhythm defers to, and it is
        // `Lessons` doing its own job: whether to do a thing at all is a
        // different question from how often.
        if !agent.lessons.worth_trying(Undertaking::Trapping) {
            return None;
        }

        let here = (agent_position.0, agent_position.1);
        let mine = self
            .world
            .snares
            .iter()
            .filter(|snare| snare.set_by == agent.id)
            .min_by_key(|snare| {
                (snare.at.0 - here.0).abs().max((snare.at.1 - here.1).abs())
            })?;

        let paces = (mine.at.0 - here.0).abs().max((mine.at.1 - here.1).abs());

        // Only what is within a walk. A line is kept near where you live, and
        // one snare the far side of the map is not a reason to cross it - the
        // same bound `walking_to_a_catch` keeps.
        if paces > Self::AS_FAR_AS_A_LINE_IS_WORTH_WALKING {
            return None;
        }

        if paces <= Self::CLOSE_ENOUGH_TO_A_SNARE {
            Some(Action::CheckSnares)
        } else {
            Some(Action::Move {
                target: (mine.at.0, mine.at.1, agent_position.2),
            })
        }
    }

    /// Nothing waiting: set another where you are standing, if there is
    /// anything pressing and there is room in the line.
    pub(in crate::analytics) fn lengthening_the_line(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> Option<Action> {
        use crate::agents::practices::Undertaking;

        // Somebody whose snares have come back empty a dozen times stops
        // setting them. On ground that will not carry anything that is the
        // right answer, and it is how an agent finds that out. It gates
        // setting rather than collecting: a catch is a catch whatever you
        // believe about trapping.
        if !agent.lessons.worth_trying(Undertaking::Trapping) {
            return None;
        }

        let here = (agent_position.0, agent_position.1);
        let mine = self
            .world
            .snares
            .iter()
            .filter(|snare| snare.set_by == agent.id)
            .count();

        let hunger = agent
            .drives
            .get(DriveType::Hunger)
            .map(|drive| drive.urgency())
            .unwrap_or(0.0);
        let putting_by = agent
            .drives
            .get(DriveType::Preparedness)
            .map(|drive| drive.urgency())
            .unwrap_or(0.0);

        if hunger.max(putting_by) < Self::WORTH_SETTING_STRING {
            return None;
        }

        if mine >= Self::A_LINE_OF_SNARES {
            return None;
        }

        if self.world.snares.iter().any(|snare| snare.at == here) {
            return None;
        }

        // And not on ground that carries nothing.
        let ground = crate::environment::fauna::AnimalManager::whose_ground(here);
        if self.world.animals.small_life.here(ground).would_carry <= 0.0 {
            return None;
        }

        Some(Action::SetSnare)
    }
}

impl Simulation {
    /// How often each person looks round for game, in half hours.
    pub(in crate::analytics) const HOW_OFTEN_THE_GAME_IS_NOTED: u32 = 4;

    /// Each grown person notes the game in sight worth hunting, and forgets
    /// a place they remembered game at and find empty (#358).
    pub(in crate::analytics) fn the_hunters_note_the_game(&mut self) {
        use crate::environment::seasons::TICKS_BETWEEN_PLANS;
        if self.current_turn % (Self::HOW_OFTEN_THE_GAME_IS_NOTED * TICKS_BETWEEN_PLANS) != 0 {
            return;
        }
        let now = self.current_turn;
        let noted: Vec<(usize, Option<(i32, i32)>)> = self
            .population
            .agents
            .iter()
            .enumerate()
            .filter(|(_, agent)| agent.state.is_alive && agent.state.life_stage.can_reproduce())
            .map(|(index, agent)| {
                let here = (agent.state.position.0, agent.state.position.1);
                let seen = self
                    .world
                    .get_animals_in_radius(here, Self::how_far_a_hunter_looks(agent))
                    .into_iter()
                    .filter(|animal| self.worth_hunting(agent, animal))
                    .filter(|animal| {
                        self.world
                            .animals
                            .get_species(&animal.species_id)
                            .is_some_and(|species| species.food_value > 0.0)
                    })
                    .map(|animal| animal.position)
                    .min_by_key(|at| ((at.0 - here.0).abs().max((at.1 - here.1).abs()), at.0, at.1));
                (index, seen)
            })
            .collect();
        for (index, seen) in noted {
            let agent = &mut self.population.agents[index];
            match seen {
                Some(at) => agent.saw_game_at(at, now),
                None => {
                    let here = (agent.state.position.0, agent.state.position.1);
                    agent
                        .game_seen
                        .retain(|(there, _)| (there.0 - here.0).abs().max((there.1 - here.1).abs()) > 10);
                }
            }
        }
    }
}
