// src/analytics/between_us/driving_off.rs
//! Driving a beast off the camp's ground (#377).
//!
//! "People protect or cull animals informally" is the first thing the
//! husbandry path lists, and until now a beast that settled on a camp could
//! only be waited out. A boar keeps the ground two cells about it and charges
//! whoever stands in it (`would_it_charge`), and a walk stops short of that
//! ground (#372), so a boar that lay down by a hearth kept everybody outside
//! its ground out of their own camp and store. On smooth seed 3 two people
//! starved three cells from home (#376).
//!
//! A beast sizes people up one at a time (`what_the_beasts_make_of_us`), and
//! a boar outweighs any one of them. What drives it off is everybody at once:
//! the grown people within earshot, each counted at what they are worth in a
//! fight as they stand, with a brand from a fire if one is burning close by.
//! Set against what the beast would need to turn and face them, the same
//! reckoning a beast makes, it either bolts or stands. Nobody tries it when
//! the sum says it would stand.

use crate::environment::Action;

use super::super::Simulation;

impl Simulation {
    /// How near a beast has to be to somebody's hearth before it is on the
    /// camp's ground: within sight of the fire, sixty metres.
    pub(in crate::analytics) const ON_THE_CAMPS_GROUND: i32 = 6;

    /// What a brand out of a burning fire is worth to the people waving it.
    /// Beasts are wary of fire, and a boar that stands up to three men stands
    /// up to fewer with torches.
    pub(in crate::analytics) const WHAT_A_BRAND_ADDS: f32 = 1.5;

    /// How near a burning fire has to be for brands to be taken from it.
    pub(in crate::analytics) const NEAR_ENOUGH_FOR_A_BRAND: i32 = 3;

    /// A beast that would charge, settled on this one's camp's ground, that
    /// those about could drive off: `DriveOff` it. Only for somebody grown
    /// and outside its ground; and only when, all together, they outweigh it.
    pub(in crate::analytics) fn a_beast_on_our_ground(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> Option<Action> {
        if agent.state.years_old() < crate::agents::Agent::OLD_ENOUGH_TO_KEEP_A_KIT
            || !agent.knows_how_to_do("DriveOff")
        {
            return None;
        }
        let home = Self::where_this_one_lives(agent);
        let here = (agent_position.0, agent_position.1);

        self.world
            .get_animals_in_radius(home, (Self::ON_THE_CAMPS_GROUND + 1) as f32)
            .into_iter()
            .filter(|animal| animal.is_alive() && !animal.is_domesticated && !animal.is_kept())
            .filter(|animal| {
                Self::within(animal.position, home, Self::ON_THE_CAMPS_GROUND)
                    && Self::within(animal.position, here, Self::AS_FAR_AS_A_BEAST_LOOKS)
                    && !Self::within(animal.position, here, Self::A_BEASTS_GROUND)
            })
            .filter(|animal| {
                self.world
                    .animals
                    .get_species(&animal.species_id)
                    .is_some_and(Self::would_it_charge)
            })
            .find(|animal| self.would_it_be_driven_off(animal, here) && self.where_it_would_bolt_to(animal, here).is_some())
            .map(|animal| Action::DriveOff { animal_id: animal.id })
    }

    /// Where a beast driven from `here` would bolt to: as far as a frightened
    /// beast gets, the way most nearly away from those driving it that has
    /// ground to stand on at the end, and as far short of that as it has to.
    ///
    /// Straight away and nowhere else, a mammoth with a lake behind it on
    /// smooth seed 10 could not go, stood where it was, and was driven off
    /// again every turn: 7,781 times in two years (#377).
    pub(in crate::analytics) fn where_it_would_bolt_to(
        &self,
        animal: &crate::environment::fauna::Animal,
        here: (i32, i32),
    ) -> Option<(i32, i32)> {
        let (dx, dy) = (animal.position.0 - here.0, animal.position.1 - here.1);
        let away = (dy as f32).atan2(dx as f32);
        let (width, height) = (self.world.grid.width as i32, self.world.grid.height as i32);
        // Straight away first, then a little to either side, then more
        [0.0f32, 0.5, -0.5, 1.0, -1.0, 1.5, -1.5]
            .iter()
            .flat_map(|turn| (1..=4).rev().map(move |quarter| (*turn, quarter)))
            .find_map(|(turn, quarter)| {
                let bolt = (Self::HOW_FAR_A_FRIGHTENED_BEAST_GETS * quarter / 4) as f32;
                let heading = away + turn;
                let to = (
                    ((animal.position.0 as f32 + heading.cos() * bolt) as i32).clamp(0, width - 1),
                    ((animal.position.1 as f32 + heading.sin() * bolt) as i32).clamp(0, height - 1),
                );
                let off_the_camp = !Self::within(to, animal.position, Self::ON_THE_CAMPS_GROUND + Self::A_BEASTS_GROUND);
                (off_the_camp && self.is_passable_tile(to.0, to.1)).then_some(to)
            })
    }

    /// Whether the grown people about `here`, all together, would drive this
    /// beast off: what they are worth in a fight, as they stand and with any
    /// brands, against what it would take for it to stand up to them.
    pub(in crate::analytics) fn would_it_be_driven_off(
        &self,
        animal: &crate::environment::fauna::Animal,
        here: (i32, i32),
    ) -> bool {
        let Some(species) = self.world.animals.get_species(&animal.species_id) else {
            return false;
        };
        let nerve = species.behavior.how_readily_it_stands_its_ground();
        if nerve <= 0.0 {
            return true;
        }
        let it_is_worth = Self::what_a_beast_is_worth_in_a_fight(
            animal.current_health,
            species.health,
            species.attack_damage,
        );
        self.what_those_about_are_worth(here) >= it_is_worth * nerve * Self::WHAT_IT_TAKES_TO_TURN_AND_FACE
    }

    /// What the grown people within earshot of `here` are worth to a beast,
    /// all together: each at what one person is worth, more with something
    /// in the hand, less for being hurt; and more again with a fire burning
    /// close by to take brands from.
    pub(in crate::analytics) fn what_those_about_are_worth(&self, here: (i32, i32)) -> f32 {
        let all_of_us: f32 = self
            .population
            .agents
            .iter()
            .filter(|them| them.state.is_alive)
            .filter(|them| them.state.years_old() >= crate::agents::Agent::OLD_ENOUGH_TO_KEEP_A_KIT)
            .filter(|them| {
                Self::within((them.state.position.0, them.state.position.1), here, Self::WITHIN_TALKING_DISTANCE)
            })
            .map(|them| {
                let armed = them
                    .what_i_have_to_work_with(crate::agents::SkillType::MeleeCombat)
                    .is_some();
                Self::WHAT_A_PERSON_IS_WORTH_TO_A_BEAST
                    * if armed { Self::WHAT_A_SPEAR_ADDS } else { 1.0 }
                    * Self::what_shape_i_am_in(them)
            })
            .sum();
        let brands = self
            .nearest_fire_from((here.0, here.1, 0), Self::NEAR_ENOUGH_FOR_A_BRAND, true)
            .is_some();
        all_of_us * if brands { Self::WHAT_A_BRAND_ADDS } else { 1.0 }
    }

    /// `Action::DriveOff`: the beast bolts from those driving it, as far as a
    /// frightened beast gets, if all together they outweigh it; and stands
    /// its ground if not.
    pub(in crate::analytics) fn driving_off(&mut self, animal_id: &uuid::Uuid, agent_index: usize) -> crate::environment::ActionResult {
        use crate::environment::fauna::AnimalState;
        use crate::environment::ActionResult;

        let here = {
            let at = self.population.agents[agent_index].state.position;
            (at.0, at.1)
        };
        let Some(animal) = self.world.animals.get(animal_id).filter(|animal| animal.is_alive()).cloned() else {
            return ActionResult::failure("Nothing there to drive off".to_string());
        };
        if !Self::within(animal.position, here, Self::AS_FAR_AS_A_BEAST_LOOKS) {
            return ActionResult::failure("Too far off to drive".to_string());
        }
        if !self.would_it_be_driven_off(&animal, here) {
            *self.what_a_threat_came_to.entry("drive off: it stood its ground".to_string()).or_insert(0) += 1;
            return ActionResult::failure("It stood its ground".to_string()).with_energy_cost(5.0);
        }

        let landed = self.where_it_would_bolt_to(&animal, here);
        if landed.is_none() {
            *self.what_a_threat_came_to.entry("drive off: nowhere for it to go".to_string()).or_insert(0) += 1;
            return ActionResult::failure("It had nowhere to go".to_string()).with_energy_cost(5.0);
        }

        if let Some(beast) = self.world.animals.get_mut(animal_id) {
            beast.state = AnimalState::Fleeing { from_position: here };
            beast.state_timer = Self::HOW_LONG_A_BEAST_KEEPS_ITS_NERVE;
            if let Some(to) = landed {
                beast.position = to;
            }
            beast.use_stamina(Self::WHAT_BOLTING_COSTS_A_BEAST);
        }
        self.world.beasts_driven_off += 1;

        // And everybody who was about knows a beast came onto the camp, which
        // is what sets a camp thinking of a ring of thorn (#378)
        let now = self.current_turn;
        let called = self
            .world
            .animals
            .get_species(&animal.species_id)
            .map(|species| species.name.clone())
            .unwrap_or_default();
        for them in self.population.agents.iter_mut().filter(|them| {
            them.state.is_alive
                && Self::within((them.state.position.0, them.state.position.1), here, Self::WITHIN_TALKING_DISTANCE)
        }) {
            them.exploration_knowledge.saw_danger(
                crate::world::Position::new(animal.position.0, animal.position.1),
                &called,
                1.0,
                now,
            );
        }
        *self.what_a_threat_came_to.entry("drive off: it bolted".to_string()).or_insert(0) += 1;

        ActionResult::success()
            .with_drive_change(crate::core::DriveType::Protection, -0.3)
            .with_energy_cost(10.0)
            .with_message("Drove it off".to_string())
    }
}

impl Simulation {
    /// How bad a camp has to remember it going, a beast on its ground or
    /// near it, before anybody rings it with thorn: a warning from a boar
    /// is the whole of the scale and fades over the weeks after.
    pub(in crate::analytics) const A_CAMP_TROUBLED_ENOUGH_FOR_THORNS: f32 = 0.3;

    /// The next of laying a thorn ring round this one's camp (#378): brush
    /// laid if there is any in hand and the ring is not closed, or fetched
    /// from within a forage. Only at a camp this one remembers a beast
    /// troubling, or where a ring is already begun.
    pub(in crate::analytics) fn a_thorn_hedge_to_lay(
        &self,
        agent: &crate::agents::Agent,
        here: (i32, i32, i32),
    ) -> Option<Action> {
        if agent.state.years_old() < crate::agents::Agent::OLD_ENOUGH_TO_KEEP_A_KIT
            || !agent.knows_how_to_do("PutUpThornHedge")
        {
            return None;
        }
        let home = Self::where_this_one_lives(agent);
        let now = self.current_turn;
        let begun = self.world.thorn_hedge_round(home).map(|which| &self.world.thorn_hedges[which]);
        if begun.is_some_and(|hedge| hedge.closed_on.is_some()) {
            return None;
        }
        // Anywhere on the camp's ground, which a remembered danger covers in
        // patches three cells wide
        let troubled = [-3, 0, 3]
            .iter()
            .flat_map(|dx| [-3, 0, 3].iter().map(move |dy| (*dx, *dy)))
            .map(|(dx, dy)| {
                agent
                    .exploration_knowledge
                    .how_bad_is_it_there(crate::world::Position::new(home.0 + dx, home.1 + dy), now)
            })
            .fold(0.0f32, f32::max)
            >= Self::A_CAMP_TROUBLED_ENOUGH_FOR_THORNS;
        if begun.is_none() && !troubled {
            return None;
        }
        if agent.how_many_i_have("wood") > 0 {
            return Some(Action::PutUpThornHedge);
        }
        self.nearest_resource_within(agent, here, Self::FORAGE_RADIUS, |resource| {
            resource.resource_type == crate::world::ResourceType::Wood
        })
        .map(|_| Action::Gather { resource_type: "wood".to_string() })
    }

    /// `Action::PutUpThornHedge`: the brush in hand laid into the ring round
    /// this one's camp, begun if it was not, and closed when it holds what a
    /// ring takes.
    pub(in crate::analytics) fn laying_a_thorn_hedge(&mut self, agent_index: usize, turn_now: u32) -> crate::environment::ActionResult {
        use crate::environment::ActionResult;
        use crate::world::ThornHedge;

        let agent = &self.population.agents[agent_index];
        if !agent.knows_how_to_do("PutUpThornHedge") {
            return ActionResult::failure("Nobody here has hedged a camp".to_string());
        }
        let home = Self::where_this_one_lives(agent);
        let here = (agent.state.position.0, agent.state.position.1);
        if !Self::within(here, home, ThornHedge::HOW_FAR_IT_GOES_ROUND) {
            return ActionResult::failure("Not at the camp".to_string());
        }
        let wood = agent.how_many_i_have("wood");
        if wood == 0 {
            return ActionResult::failure("No brush in hand".to_string());
        }

        let which = match self.world.thorn_hedge_round(home) {
            Some(which) => which,
            None => {
                self.world.thorn_hedges.push(ThornHedge {
                    around: crate::world::Position::new(home.0, home.1),
                    started: turn_now,
                    laid: 0,
                    closed_on: None,
                });
                self.world.thorn_hedges.len() - 1
            }
        };
        let hedge = &mut self.world.thorn_hedges[which];
        if hedge.closed_on.is_some() {
            return ActionResult::failure("The camp is hedged already".to_string());
        }
        let laying = wood.min(ThornHedge::WHAT_IT_TAKES - hedge.laid);
        hedge.laid += laying;
        let closed = hedge.laid >= ThornHedge::WHAT_IT_TAKES;
        if closed {
            hedge.closed_on = Some(turn_now);
            self.world.thorn_hedges_closed += 1;
        }

        let agent = &mut self.population.agents[agent_index];
        agent.inventory.remove_item("wood", laying);
        agent.skills.practise(crate::agents::skills::SkillType::Construction, 3, turn_now);

        ActionResult::success()
            .with_drive_change(crate::core::DriveType::Protection, -0.2)
            .with_energy_cost(6.0)
            .with_message(if closed {
                "Closed the thorn ring round the camp".to_string()
            } else {
                format!("Laid {laying} of brush into the thorn ring")
            })
    }
}
