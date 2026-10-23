// src/analytics/wanting/journey.rs
//! Journeys beyond a day's forage, and the first thing anybody makes one for:
//! salt (#356).

use super::super::Simulation;
use crate::environment::Action;

impl Simulation {
    /// How far somebody will go for salt they know of: anywhere they could
    /// walk to and back inside a few days.
    pub(in crate::analytics) const HOW_FAR_ONE_GOES_FOR_SALT: u32 = 2_000;

    /// What is brought back from a salt flat: a load to lick from and salt
    /// food with for a season.
    pub(in crate::analytics) const A_LOAD_OF_SALT: u32 = 8;

    /// How long a journey goes on before it is given up, in ticks: four days.
    pub(in crate::analytics) const A_JOURNEY_GIVES_UP_AFTER: u32 =
        4 * crate::environment::seasons::TICKS_PER_DAY;

    /// What a journey for salt asks of the turn (#356).
    ///
    /// Somebody craving salt with none on them, who knows where some lies,
    /// goes for it: gathered where it lies within a forage, or else a journey
    /// - food packed, a vessel filled (`water_for_the_walk`), a night or two
    /// slept out - there and home again with a load. A journey under way is
    /// kept to across turns and nights until it is home or given up.
    ///
    /// Anything that will not wait comes first: danger, hunger, thirst, the
    /// cold and the weather, and the night's sleep after it.
    pub(in crate::analytics) fn what_the_salt_asks(
        &mut self,
        agent_index: usize,
        action: Action,
        running_away: bool,
    ) -> Action {
        if running_away {
            return action;
        }
        let agent = &self.population.agents[agent_index];
        if agent.state.years_old() < Self::OLD_ENOUGH_TO_COOK || self.this_will_not_wait(agent_index, &action) {
            return action;
        }
        let here = (agent.state.position.0, agent.state.position.1);
        let now = self.current_turn;

        if let Some(journey) = agent.journey.clone() {
            return self.keeping_to_the_journey(agent_index, journey, action, here, now);
        }

        if !agent.craves_salt() || agent.how_many_i_have("salt") > 0 {
            return action;
        }
        let from = crate::world::Position::new(here.0, here.1);
        let Some(salt) = self
            .nodes_known_to(agent, from, Self::HOW_FAR_ONE_GOES_FOR_SALT)
            .filter(|node| node.resource_type == crate::world::ResourceType::Salt && node.anything_to_take())
            .map(|node| node.position)
            .min_by_key(|at| from.distance_to(at))
        else {
            return action;
        };
        if from.distance_to(&salt) <= Self::FORAGE_RADIUS {
            *self.what_a_threat_came_to.entry("salt: gathered near".to_string()).or_insert(0) += 1;
            return Action::Gather { resource_type: "salt".to_string() };
        }

        // Food for the way first, as for any walk past the next meal
        let target = (salt.x, salt.y, agent.state.position.2);
        if let Some(first) = self.what_to_take_along(agent_index, target, crate::core::DriveType::Hunger) {
            return first;
        }
        self.population.agents[agent_index].journey = Some(crate::agents::Journey {
            to: (salt.x, salt.y),
            for_what: "salt".to_string(),
            set_out: now,
            homeward: false,
        });
        *self.what_a_threat_came_to.entry("salt: set out on a journey".to_string()).or_insert(0) += 1;
        Action::Move { target }
    }

    /// A journey already under way: walked on, the thing gathered once
    /// there, and walked home.
    fn keeping_to_the_journey(
        &mut self,
        agent_index: usize,
        journey: crate::agents::Journey,
        action: Action,
        here: (i32, i32),
        now: u32,
    ) -> Action {
        let z = self.population.agents[agent_index].state.position.2;
        let from = crate::world::Position::new(here.0, here.1);
        let to = crate::world::Position::new(journey.to.0, journey.to.1);
        if now.saturating_sub(journey.set_out) > Self::A_JOURNEY_GIVES_UP_AFTER
            || !self.world.can_walk_between(here, journey.to)
        {
            *self.what_a_threat_came_to.entry("journey: given up".to_string()).or_insert(0) += 1;
            self.population.agents[agent_index].journey = None;
            return action;
        }

        if journey.homeward {
            if from.distance_to(&to) <= Self::NEAR_ENOUGH_TO_SLEEP_WITH_THE_OTHERS as u32 {
                *self.what_a_threat_came_to.entry("journey: home again".to_string()).or_insert(0) += 1;
                self.population.agents[agent_index].journey = None;
                return action;
            }
            return Action::Move { target: (journey.to.0, journey.to.1, z) };
        }

        // There: gathered until the load is in the pack or the place gives
        // out, and then home
        let agent = &self.population.agents[agent_index];
        let loaded = agent.how_many_i_have(&journey.for_what) >= Self::A_LOAD_OF_SALT;
        let still_there = self
            .nodes_known_to(agent, to, 1)
            .any(|node| node.position == to && node.anything_to_take());
        if loaded || !still_there {
            let home = Self::where_this_one_lives(agent);
            if let Some(walk) = self.population.agents[agent_index].journey.as_mut() {
                walk.homeward = true;
                walk.to = (home.0, home.1);
            }
            *self.what_a_threat_came_to.entry("journey: turned for home".to_string()).or_insert(0) += 1;
            return Action::Move { target: (home.0, home.1, z) };
        }
        if from.distance_to(&to) <= Self::FORAGE_RADIUS {
            return Action::Gather { resource_type: journey.for_what };
        }
        Action::Move { target: (journey.to.0, journey.to.1, z) }
    }

    /// Whether what this one is about is something that will not wait for a
    /// walk: danger, hunger or thirst pressing, the cold, the weather, or a
    /// meal.
    fn this_will_not_wait(&self, agent_index: usize, action: &Action) -> bool {
        use crate::core::DriveType;
        let agent = &self.population.agents[agent_index];
        let pressing = |drive: DriveType| agent.drives.get(drive).is_some_and(|it| it.is_active());
        agent.emotions.threatened_by_any_of(|source| self.is_it_about(agent_index, source))
            || agent.state.is_starving()
            || agent.state.is_dehydrated()
            || pressing(DriveType::Hunger)
            || pressing(DriveType::Thirst)
            || agent.body_temperature.is_too_cold()
            || !agent.exposure_status.active_exposures.is_empty()
            || matches!(action, Action::Eat { .. } | Action::Cook { .. } | Action::Sleep { .. })
    }
}
