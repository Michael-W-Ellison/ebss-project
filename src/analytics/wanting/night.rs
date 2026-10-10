// src/analytics/wanting/night.rs
//! What the night asks: that people sleep in it.
//!
//! Nothing in the model knew there was a night. People slept when the Rest
//! drive won a turn, which came to about two and a half half-hours a day, in
//! scraps, wherever they stood; so they lived nine tenths fatigued, and on
//! their one fertile day in a month the person who would have been the other
//! parent was usually somewhere else on the map (#298). People sleep at
//! night, and mostly together. Part of the decision layer - see [`super`].

use super::super::Simulation;
use crate::environment::Action;

impl Simulation {
    /// The most a night's sleep runs to, and the least, in hours.
    ///
    /// Waking at first light, and going to bed long enough before it to have
    /// slept: a long winter night is a longer sleep, not a sixteen-hour one.
    pub(in crate::analytics) const THE_LONGEST_NIGHTS_SLEEP: f32 = 9.0;
    pub(in crate::analytics) const THE_SHORTEST_NIGHTS_SLEEP: f32 = 7.0;

    /// How long before bed people start back to where the others sleep.
    pub(in crate::analytics) const HEADING_HOME_FOR: f32 = 3.0;

    /// The longest one sleep runs before the sleeper stirs, in turns.
    ///
    /// Only danger cuts a held action short (see `one_persons_turn`), so a
    /// night slept in one piece left the weather that came on during it to
    /// be met at first light. Two adults in four settlements died of the
    /// weather in four years with the night slept whole, where none had
    /// before (#299).
    pub(in crate::analytics) const A_STRETCH_OF_SLEEP: u32 = 4;

    /// Near enough to the camp to sleep in it, in cells.
    pub(in crate::analytics) const NEAR_ENOUGH_TO_SLEEP_WITH_THE_OTHERS: i32 = 20;

    /// Where in its night the world is: `(asleep, heading home, turns of
    /// sleep left before first light)`.
    pub(in crate::analytics) fn where_in_the_night_it_is(&self) -> (bool, bool, u32) {
        let calendar = &self.world.climate.calendar;
        let day = calendar.current_season().day_length();
        let sunrise = 12.0 - day / 2.0;
        let sleep = (24.0 - day - 2.0)
            .clamp(Self::THE_SHORTEST_NIGHTS_SLEEP, Self::THE_LONGEST_NIGHTS_SLEEP);
        let bedtime = (sunrise - sleep).rem_euclid(24.0);
        let hours_since = |from: f32| (calendar.time_of_day - from).rem_euclid(24.0);

        let asleep = hours_since(bedtime) < sleep;
        let heading_home =
            !asleep && hours_since((bedtime - Self::HEADING_HOME_FOR).rem_euclid(24.0)) < Self::HEADING_HOME_FOR;
        let until_light = (sunrise - calendar.time_of_day).rem_euclid(24.0);
        let turns_left = (until_light / calendar.hours_per_turn()).ceil().max(1.0) as u32;
        (asleep, heading_home, turns_left)
    }

    /// The nearest clean ground out of the weather within
    /// `NEAR_ENOUGH_TO_SLEEP_WITH_THE_OTHERS`, for somebody about to sleep
    /// where they are not: under a finished roof, or among trees.
    ///
    /// `None` when they are covered already, or there is no such ground in
    /// reach. Foul ground is passed over, or a sleeper would be sent onto a
    /// midden under a roof and straight off it again.
    pub(in crate::analytics) fn somewhere_out_of_the_weather(
        &self,
        from: (i32, i32, i32),
    ) -> Option<(i32, i32, i32)> {
        use crate::world::Position;

        let fit = |at: &Position| {
            self.world.is_out_of_the_weather_at(at)
                && self
                    .world
                    .grid
                    .get_tile(at)
                    .is_some_and(|tile| !tile.soil.is_foul() && tile.terrain.is_walkable())
        };
        if fit(&Position::new(from.0, from.1)) {
            return None;
        }

        let reach = Self::NEAR_ENOUGH_TO_SLEEP_WITH_THE_OTHERS;
        let mut best: Option<((i32, i32), i32)> = None;
        for dy in -reach..=reach {
            for dx in -reach..=reach {
                let how_far = dx.abs() + dy.abs();
                if best.is_some_and(|(_, nearest)| how_far >= nearest) {
                    continue;
                }
                let there = Position::new(from.0 + dx, from.1 + dy);
                if fit(&there) {
                    best = Some(((there.x, there.y), how_far));
                }
            }
        }
        best.map(|((x, y), _)| (x, y, from.2))
    }

    /// An evening at home: a fire, and supper cooked on it.
    ///
    /// **Why the evening has a fire in it.** With a third of the day asleep,
    /// the moments somebody was hungry, holding raw fish and carrying the ten
    /// sticks a new fire wants stopped coinciding: one person given wood, fish
    /// and a cook's hands lit nothing in twenty-five days, where awake all
    /// night they had a fire going on the second (#299). The evening is when
    /// people at a camp light a fire and cook on it, and the fire is warmth to
    /// sleep by as well. Only for somebody with something worth cooking: a fire
    /// burning beside them is cooked at, one burning near is walked to, else
    /// one is lit if the wood is in hand, or the wood fetched if trees are near.
    pub(in crate::analytics) fn the_evening_at_home(&self, agent_index: usize) -> Option<Action> {
        let agent = &self.population.agents[agent_index];
        let here = agent.state.position;
        if agent.state.years_old() < Self::OLD_ENOUGH_TO_COOK || !Self::has_food_worth_cooking(agent) {
            return None;
        }
        if self.nearest_fire_from(here, Self::FIRE_REACH, true).is_some() {
            return Some(Action::Cook { food_type: "generic".to_string() });
        }
        if let Some((_, fire)) =
            self.nearest_fire_from(here, Self::NEAR_ENOUGH_TO_SLEEP_WITH_THE_OTHERS, true)
        {
            return Some(Action::Move { target: fire });
        }
        if agent.inventory.has_item("wood", self.wood_a_fire_here_takes(agent_index)) {
            return Some(Action::LightFire);
        }
        self.nearest_resource_within(agent, here, Self::FORAGE_RADIUS, |resource| {
            resource.resource_type == crate::world::ResourceType::Wood
        })
        .map(|_| Action::Gather { resource_type: "wood".to_string() })
    }

    /// Sleep at night, and be back with the others by then.
    ///
    /// Asked of the action already chosen, after the errand has had its say,
    /// so that a walk under way is set aside for the night rather than walked
    /// on in the dark. It gives way to anything that will not wait: somebody
    /// running for it, starving, parched or freezing does what that asks;
    /// supper is eaten, cooked or a fire lit for it, and somebody hungry with
    /// food on them eats before they lie down. An infant is wherever its
    /// people are and is not asked.
    pub(in crate::analytics) fn what_the_night_asks(
        &mut self,
        agent_index: usize,
        action: Action,
        running_away: bool,
    ) -> Action {
        if running_away {
            return action;
        }
        let (asleep, heading_home, turns_left) = self.where_in_the_night_it_is();
        if !asleep && !heading_home {
            return action;
        }

        let agent = &self.population.agents[agent_index];
        let let_be = if agent.state.life_stage == crate::agents::LifeStage::Infant {
            Some("night: let be, an infant")
        } else if agent.state.is_starving() || agent.state.is_dehydrated() {
            Some("night: let be, starving or parched")
        } else if agent.body_temperature.is_too_cold() {
            Some("night: let be, too cold")
        } else if !agent.exposure_status.active_exposures.is_empty() {
            Some("night: let be, the weather")
        } else if matches!(action, Action::Eat { .. } | Action::Cook { .. } | Action::LightFire) {
            Some("night: let be, supper")
        } else if matches!(action, Action::Treat { .. }) {
            // The Rest drive's way to bed takes a remedy first, if there is
            // one in the pack.
            Some("night: let be, a remedy")
        } else if matches!(action, Action::Sleep { .. }) {
            Some("night: asleep already")
        } else {
            None
        };
        if let Some(why) = let_be {
            *self.what_a_threat_came_to.entry(why.to_string()).or_insert(0) += 1;
            return action;
        }
        let here = agent.state.position;

        // **Supper first.** Somebody sent to bed hungry with food in the pack
        // slept nine hours on it and woke starving, and a starving man eats his
        // fish raw rather than wait on a fire - so nobody who went to bed
        // hungry ever cooked again. Hungry at bedtime with something to eat,
        // they eat it, and then they sleep (#299).
        let hungry = agent
            .drives
            .get(crate::core::DriveType::Hunger)
            .is_some_and(|hunger| hunger.is_active());
        if asleep
            && hungry
            && crate::agents::storage_integration::count_food_in_inventory(&agent.inventory) > 0
        {
            *self.what_a_threat_came_to.entry("night: supper first".to_string()).or_insert(0) += 1;
            return Action::Eat { food_type: "generic".to_string() };
        }

        // **Nobody goes to bed thirsty**, nor hungry with nothing in hand
        // to eat. A need left unanswered does not wait for morning: it puts
        // every need that stands behind it to sleep as well (see
        // `DriveState::tick`), and the wish for a child stands behind
        // hunger, thirst, rest and safety. Sent to bed as they were, people
        // on seeds 0 and 1 spent two to three times as many night turns
        // thirsty in their second year as they had before there was a night,
        // and fertile days that came all clear fell from 29 and 9 to 4 and 2
        // (#299).
        // They see to it, and then they sleep.
        let thirsty = agent
            .drives
            .get(crate::core::DriveType::Thirst)
            .is_some_and(|thirst| thirst.is_active());
        if thirsty || (asleep && hungry) {
            *self
                .what_a_threat_came_to
                .entry(if thirsty { "night: let be, thirsty" } else { "night: let be, hungry" }.to_string())
                .or_insert(0) += 1;
            return action;
        }

        // **Nobody lies down in it.** The Rest drive's own way to bed steps
        // off foul ground first (see `somewhere_that_does_not_stink`); sent
        // to bed by the clock, everybody slept where they stood, which was
        // where everybody else had been all evening. Foul ground killed two
        // in four settlements in four years, where it had killed none
        // before (#299).
        // Under cover, a sleeper stays put even on foul ground: the ground
        // tells one day in twenty at its very worst, and the weather does not
        // wait that long.
        let under_cover = self
            .world
            .is_out_of_the_weather_at(&crate::world::Position::new(here.0, here.1));
        let off_the_midden = if asleep && !under_cover {
            self.somewhere_that_does_not_stink(here)
        } else {
            None
        };

        // **And nobody sleeps out in the rain** when there is a roof or the
        // trees within a short walk. Awake, people went in when the weather
        // came on; asleep where they stood, the rain, the wind and the storm
        // did their damage all night without a word, because none of them
        // shows as a hurt until the body is cold. Over nine years on seeds
        // 0-3 the weather killed fifteen where it had killed five (#299).
        let out_of_the_weather = if asleep { self.somewhere_out_of_the_weather(here) } else { None };

        let instead = if let Some(cover) = out_of_the_weather {
            Action::Move { target: cover }
        } else if let Some(clean) = off_the_midden {
            Action::Move { target: clean }
        } else if asleep {
            // Until first light, a stretch at a time: a night is a sleep,
            // not scraps of one, but nothing wakes a sleeper but danger, and
            // rain or a frost that came on in the small hours went unanswered
            // until morning.
            Action::Sleep { duration: turns_left.min(Self::A_STRETCH_OF_SLEEP) }
        } else {
            match self.where_this_one_sleeps(agent_index) {
                Some(home)
                    if (home.0 - here.0).abs().max((home.1 - here.1).abs())
                        > Self::NEAR_ENOUGH_TO_SLEEP_WITH_THE_OTHERS =>
                {
                    Action::Move { target: (home.0, home.1, here.2) }
                }
                Some(_) => match self.the_evening_at_home(agent_index) {
                    Some(evening) => evening,
                    None => {
                        *self
                            .what_a_threat_came_to
                            .entry("night: home already".to_string())
                            .or_insert(0) += 1;
                        return action;
                    }
                },
                None => {
                    *self.what_a_threat_came_to.entry("night: no camp to head for".to_string()).or_insert(0) += 1;
                    return action;
                }
            }
        };

        *self
            .what_a_threat_came_to
            .entry(
                match (&instead, asleep) {
                    (Action::Move { .. }, true) if out_of_the_weather.is_some() => "night: went in out of the weather",
                    (Action::Move { .. }, true) => "night: stepped off foul ground",
                    (_, true) => "night: slept",
                    (Action::Move { .. }, false) => "night: headed home",
                    (Action::Cook { .. }, _) => "night: cooked supper",
                    (Action::LightFire, _) => "night: lit the evening fire",
                    _ => "night: fetched wood for the fire",
                }
                .to_string(),
            )
            .or_insert(0) += 1;
        // Lying down at home makes this one's hearth wherever the camp is now.
        if asleep {
            self.settle_the_hearth(agent_index);
        }
        if self.population.agents[agent_index].errand.is_some() {
            self.set_the_errand_aside(agent_index, instead)
        } else {
            instead
        }
    }
}
