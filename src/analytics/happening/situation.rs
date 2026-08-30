// src/analytics/happening/situation.rs
//! Reading the world, so that a drive can rise on a condition rather than on
//! a clock.
//!
//! What is around each agent this tick, what the afternoon was like, and what
//! everybody makes of their provisions against the winter coming.
//!
//! Part of what happens whether or not anybody decides anything - see
//! [`super`]. Called from [`crate::analytics::turn`], in the order argued over
//! there.

use super::super::Simulation;
use crate::agents::practices::Circumstance;
use crate::core::DriveType;

impl Simulation {
    /// What the world is doing where this agent is standing.
    ///
    /// Nobody chooses these and nobody is asked about them. They are written
    /// down against every attempt an agent makes, and what the agent works out
    /// afterwards is which of them go with a thing working - see
    /// [`crate::agents::practices::Circumstance`].
    ///
    /// This is the whole of the mechanism by which a lesson can be about a
    /// situation nobody named. Everything that had to be a rule or a discovery
    /// flag before - laying fish out only pays under a clear sky, firing clay
    /// only works at a fire, greens are a spring thing - is in principle
    /// reachable from here without anybody writing it down, because the sky,
    /// the fire and the season are all in this list and the arithmetic that
    /// reads them does not know or care what any of them is for.
    pub(in crate::analytics) fn what_it_is_like_here(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> Vec<Circumstance> {
        use crate::environment::seasons::Season;
        use crate::world::Position;

        let mut here = Vec::with_capacity(4);

        if self.is_the_sky_clear() {
            here.push(Circumstance::ClearSky);
        } else if self
            .world
            .climate
            .weather
            .weather_type
            .precipitation_intensity()
            > 0.0
        {
            here.push(Circumstance::Raining);
        }

        if self
            .nearest_fire_from(agent_position, Self::FIRE_REACH, true)
            .is_some()
        {
            here.push(Circumstance::AFireToHand);
        }

        // Standing under one, not within a walk of one: this is a fact about
        // the afternoon, and a roof across the camp keeps nothing off you.
        if self.world.buildings.iter().any(|building| {
            building.position.x == agent_position.0 && building.position.y == agent_position.1
        }) {
            here.push(Circumstance::UnderARoof);
        }

        if self.population.agents.iter().any(|other| {
            other.state.is_alive
                && other.id != agent.id
                && (other.state.position.0 - agent_position.0).abs() <= Self::WITHIN_SIGHT
                && (other.state.position.1 - agent_position.1).abs() <= Self::WITHIN_SIGHT
        }) {
            here.push(Circumstance::OtherPeopleAbout);
        }

        let by_water = (-Self::WITHIN_A_FEW_PACES..=Self::WITHIN_A_FEW_PACES).any(|dy| {
            (-Self::WITHIN_A_FEW_PACES..=Self::WITHIN_A_FEW_PACES).any(|dx| {
                self.world
                    .grid
                    .get_tile(&Position::new(agent_position.0 + dx, agent_position.1 + dy))
                    .is_some_and(|tile| tile.terrain.is_aquatic())
            })
        });
        if by_water {
            here.push(Circumstance::ByWater);
        }

        here.push(match self.world.climate.current_season() {
            Season::Spring => Circumstance::InSpring,
            Season::Summer => Circumstance::InSummer,
            Season::Fall => Circumstance::InAutumn,
            Season::Winter => Circumstance::InWinter,
        });

        here
    }

    /// How far off somebody else still counts as being about.
    pub(in crate::analytics) const WITHIN_SIGHT: i32 = 6;

    /// And how far off water still counts as being here.
    pub(in crate::analytics) const WITHIN_A_FEW_PACES: i32 = 2;

    /// How far somebody will walk to a store, either to fill it or to draw on
    /// it.
    pub(in crate::analytics) const WORTH_WALKING_TO_THE_STORE: u32 = 14;

    pub(in crate::analytics) fn within(one: (i32, i32), other: (i32, i32), paces: i32) -> bool {
        (one.0 - other.0).abs().max((one.1 - other.1).abs()) <= paces
    }

    /// What a person on their own is worth to something with teeth.
    ///
    /// About a wolf. People are slow, soft and have no claws, and what makes
    /// them dangerous is what is in their hands.
    pub(in crate::analytics) const WHAT_A_PERSON_IS_WORTH_TO_A_BEAST: f32 = 0.6;

    /// And what something in those hands adds.
    pub(in crate::analytics) const WHAT_A_SPEAR_ADDS: f32 = 2.2;

    /// How much better than the thing coming at it an animal has to be before
    /// it turns and faces it.
    ///
    /// Above one, because running is the safe answer and a wild thing that
    /// gets this wrong does not get to be wrong twice.
    pub(in crate::analytics) const WHAT_IT_TAKES_TO_TURN_AND_FACE: f32 = 1.1;

    /// How far a beast looks about itself.
    pub(in crate::analytics) const AS_FAR_AS_A_BEAST_LOOKS: i32 = 7;

    /// How far a frightened animal gets in one turn. Further than a person:
    /// a deer outruns anything on two legs.
    pub(in crate::analytics) const HOW_FAR_A_FRIGHTENED_BEAST_GETS: i32 = 6;

    /// What that costs it.
    pub(in crate::analytics) const WHAT_BOLTING_COSTS_A_BEAST: f32 = 3.0;

    /// How long an animal goes on being frightened before it settles again.
    pub(in crate::analytics) const HOW_LONG_A_BEAST_KEEPS_ITS_NERVE: u32 = 3;

    /// How close something that would eat you counts as close
    pub(in crate::analytics) const A_THREAT_NEARBY: i32 = 8;

    /// How far an agent looks when judging whether the ground round about is
    /// still bearing
    pub(in crate::analytics) const GROUND_ROUND_ABOUT: u32 = 10;

    /// What a tile of ground within reach ought to be carrying before an agent
    /// stops worrying about next year's food
    pub(in crate::analytics) const A_TILE_WORTH_HAVING: f32 = 25.0;

    /// Tell every agent what the world around it is doing.
    ///
    /// The drives are specified by the conditions that raise them - "hostile
    /// entity proximity", "nightfall", "others building", "crop depletion" -
    /// and half of those are things only the world knows. This gathers them
    /// once a tick per agent. The agent folds in what it knows about itself
    /// when its own drives are ticked, one tick later, which is near enough:
    /// nothing here changes faster than an agent can walk.
    pub(in crate::analytics) fn read_the_situation(&mut self) {
        use crate::world::{Position, TerrainType};

        let night = !self.world.climate.is_daytime();
        let foul_weather = self.world.climate.weather.weather_type.precipitation_intensity() > 0.0
            || self.world.climate.weather.effective_wind_speed() > 8.0;

        // Where the predators are, and where anybody is building
        let hunters: Vec<(i32, i32)> = self
            .world
            .animals
            .get_all()
            .iter()
            .filter(|animal| animal.is_alive() && !animal.is_domesticated)
            .filter(|animal| {
                self.world
                    .animals
                    .get_species(&animal.species_id)
                    .map(|species| species.attack_damage > 0.0)
                    .unwrap_or(false)
            })
            .map(|animal| (animal.position.0, animal.position.1))
            .collect();

        let building_sites: Vec<(i32, i32)> = self
            .world
            .buildings
            .iter()
            .filter(|building| !building.is_completed())
            .map(|building| (building.position.x, building.position.y))
            .collect();

        let current_tick = self.current_tick;

        // Small children, by whose parent they are
        let young: Vec<(Vec<uuid::Uuid>, (i32, i32, i32))> = self
            .population
            .agents
            .iter()
            .filter(|agent| agent.state.is_alive)
            .filter(|agent| {
                matches!(
                    agent.state.life_stage,
                    crate::agents::LifeStage::Infant | crate::agents::LifeStage::Child
                )
            })
            .map(|agent| (agent.parent_ids.clone(), agent.state.position))
            .collect();

        let grown: Vec<(i32, i32, i32)> = self
            .population
            .agents
            .iter()
            .filter(|agent| agent.state.is_alive)
            .map(|agent| agent.state.position)
            .collect();

        // What the ground within reach is carrying, per agent position
        let crop_at = |position: (i32, i32, i32)| -> f32 {
            let here = Position::new(position.0, position.1);
            let mut standing = 0u32;
            let mut patches = 0u32;

            for resource in &self.world.resources {
                if !resource.resource_type.is_edible() {
                    continue;
                }
                if here.distance_to(&resource.position) > Self::GROUND_ROUND_ABOUT {
                    continue;
                }
                standing += resource.amount;
                patches += 1;
            }

            if patches == 0 {
                return 0.0;
            }

            (standing as f32 / (patches as f32 * Self::A_TILE_WORTH_HAVING)).clamp(0.0, 1.0)
        };

        let mut readings = Vec::with_capacity(self.population.agents.len());

        for agent in &self.population.agents {
            if !agent.state.is_alive {
                readings.push(None);
                continue;
            }

            let position = agent.state.position;
            let near = |spot: &(i32, i32), reach: i32| {
                (spot.0 - position.0).abs().max((spot.1 - position.1).abs()) <= reach
            };

            let mine: Vec<&(Vec<uuid::Uuid>, (i32, i32, i32))> = young
                .iter()
                .filter(|(parents, _)| parents.contains(&agent.id))
                .collect();

            let child_astray = mine.iter().any(|(_, child)| {
                let strayed = (child.0 - position.0).abs().max((child.1 - position.1).abs())
                    > Self::CHILD_LEASH;
                let stalked = hunters.iter().any(|hunter| {
                    (hunter.0 - child.0).abs().max((hunter.1 - child.1).abs())
                        <= Self::DANGER_TO_A_CHILD
                });
                strayed || stalked
            });

            let here = Position::new(position.0, position.1);
            let ground = self
                .world
                .grid
                .get_tile(&here)
                .map(|tile| tile.terrain.terrain_type)
                .unwrap_or(TerrainType::Plains);

            readings.push(Some(crate::core::Surroundings {
                predator_near: hunters.iter().any(|spot| near(spot, Self::A_THREAT_NEARBY)),
                night,
                foul_weather,
                under_shelter: self
                    .world
                    .buildings
                    .iter()
                    .any(|building| building.position == here && building.is_completed()),
                recently_hurt: agent.emotions.recent_attacker(current_tick).is_some(),
                crop_near: crop_at(position),
                somewhere_to_build: crate::world::Terrain::new(ground).can_be_tilled(),
                neighbours_building: building_sites.iter().any(|spot| near(spot, 12)),
                children_to_mind: mine.len() as u32,
                child_astray,
                company: grown
                    .iter()
                    .filter(|other| **other != position)
                    .any(|other| {
                        (other.0 - position.0).abs().max((other.1 - position.1).abs()) <= 6
                    }),
            }));
        }

        for (agent, reading) in self.population.agents.iter_mut().zip(readings) {
            if let Some(reading) = reading {
                agent.surroundings = reading;
            }
        }
    }

    /// How close a predator has to be to a child before its parent runs
    pub(in crate::analytics) const DANGER_TO_A_CHILD: i32 = 10;

    /// What each agent makes of its own provisions against the winter coming.
    ///
    /// "Do I have enough supplies to survive the day? The week? The month? The
    /// winter?" Four horizons, each less frightening to fail than the last,
    /// and the answer comes out as one number that becomes the Preparedness
    /// drive - which already knows how to put food by. See
    /// `agents::provision`.
    ///
    /// What an agent can reach is its own pack and the camp's pits. A pit is
    /// the settlement's, not any one person's, so everybody counts the same
    /// store and everybody is easier for it being full: that is the whole
    /// reason a people digs one.
    pub(in crate::analytics) fn reckon_what_is_put_by(&mut self) {
        use crate::agents::provision::{WhatIsPutBy, UNITS_IN_ONE_STORED_ITEM};

        let season = self.world.climate.current_season();
        let day_of_year = (self.current_tick
            / crate::environment::seasons::TICKS_PER_DAY)
            % crate::environment::seasons::DAYS_PER_YEAR;

        let in_the_ground: f32 = self
            .world
            .pits
            .iter()
            .map(|pit| pit.how_much_is_in_it() as f32)
            .sum::<f32>()
            * UNITS_IN_ONE_STORED_ITEM;

        let mouths = self
            .population
            .agents
            .iter()
            .filter(|a| a.state.is_alive)
            .count()
            .max(1) as f32;
        let each_ones_share = in_the_ground / mouths;

        for agent in self.population.agents.iter_mut() {
            if !agent.state.is_alive {
                continue;
            }

            agent.state.winters_seen.another_day(season, day_of_year);

            let in_hand = crate::agents::storage_integration::count_food_in_inventory(
                &agent.inventory,
            ) as f32
                * UNITS_IN_ONE_STORED_ITEM;

            // And what is still coming out of the body's own stores counts:
            // somebody who has just eaten is not short of supper.
            let in_the_body = agent.state.physiology.in_the_stomach()
                + agent.state.physiology.in_the_gut();

            let reckoning = WhatIsPutBy::reckon(
                in_hand + each_ones_share + in_the_body,
                agent.state.physiology.what_i_burn_in_a_day,
                agent.state.winters_seen.how_long_a_winter_lasts(),
                day_of_year,
            )
            .of_which_in_the_body(in_the_body);

            if let Some(drive) = agent.drives.get_mut(DriveType::Preparedness) {
                drive.value = reckoning.stress();
            }
            agent.state.what_the_larder_says = Some(reckoning);
        }
    }
}
