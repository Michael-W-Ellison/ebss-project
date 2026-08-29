// src/analytics/wanting/camp.rs
//! Whether to stay, and where to go instead.
//!
//! A settlement that keeps going short of the same thing is a settlement in
//! the wrong place. This is what notices that, and what it does about it.
//!
//! Part of the decision layer - see [`super`]. Nothing here does anything: it
//! answers what would be worth doing, and hands that answer back up the ladder.

use super::super::Simulation;
use crate::core::DriveType;
use crate::environment::Action;

impl Simulation {
    /// The need this agent keeps having and keeps not getting.
    ///
    /// `denied_ticks` counts how long a drive has gone unanswered, and until
    /// now only hunger was ever read for the purpose of moving house. Thirst
    /// was the largest single failure in the whole simulation - a hundred and
    /// thirty-one thousand refusals of `Gather: No water sources nearby` in
    /// one pair of worlds - because an agent that could not find water walked
    /// to it, drank, wandered off about its business, and was thirsty again
    /// half a day later in the same dry place.
    pub(in crate::analytics) fn what_i_keep_going_short_of(agent: &crate::agents::Agent) -> Option<DriveType> {
        // Water only, and deliberately.
        //
        // Water is a fixed point on the map: it is in one place, it does not
        // run out, and camping beside it answers the need for good. Food is
        // not - it is spread about and it is *consumed*, so a people who move
        // house towards it concentrate their foraging on whatever ground they
        // land on and work it out from under themselves. Measured, letting
        // hunger move a settlement took the nutrient-loop regression from
        // passing three times in three to twice in five: farmed ground losing
        // more than half its fertility inside ten thousand ticks.
        //
        // Ranging for food and settling by water is the division the land
        // itself makes.
        [DriveType::Thirst]
            .into_iter()
            .filter(|need| {
                agent
                    .drives
                    .get(*need)
                    .map(|drive| drive.denied_ticks() >= Self::ASKED_FOR_IT_ONCE_TOO_OFTEN)
                    .unwrap_or(false)
            })
            .max_by_key(|need| {
                agent
                    .drives
                    .get(*need)
                    .map(|drive| drive.denied_ticks())
                    .unwrap_or(0)
            })
    }

    /// Go and live where the thing you keep needing is.
    ///
    /// "The agents must anticipate their future drive demands. If they
    /// consistently need water, they should camp or colonize near water."
    ///
    /// Answering a need where you stand is what every other path here does.
    /// This is the one that reads the *pattern* of a need instead of the need
    /// itself: a man who has been short of water for eight days does not want
    /// a drink, he wants to be somewhere else.
    ///
    /// It fires only once a need has been going unanswered for days, and stops
    /// the moment the agent is camped on the answer, so it moves a settlement
    /// rather than keeping it walking.
    pub(in crate::analytics) fn go_and_live_where_it_is(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> Option<Action> {
        use crate::world::ResourceType;

        // Whether to move house is a question worth asking once a day, not
        // eight times: it walks the whole resource list at sixty tiles, and a
        // people do not reconsider where they live every two hours.
        if self.current_tick % crate::environment::seasons::TICKS_PER_DAY != 0 {
            return None;
        }

        let short_of = Self::what_i_keep_going_short_of(agent)?;

        let wanted = match short_of {
            DriveType::Thirst => ResourceType::Water,
            _ => ResourceType::Food,
        };

        // The nearest place that answers it, however far off - this is a
        // decision to move house, so the ordinary foraging radius does not
        // apply
        let there = self.nearest_resource_within(agent_position, Self::HOW_FAR_A_PEOPLE_WILL_MOVE, |resource| {
            resource.resource_type == wanted
                || (wanted == ResourceType::Food
                    && Self::edible_item_for(resource.resource_type).is_some())
        })?;

        let paces = (there.x - agent_position.0)
            .abs()
            .max((there.y - agent_position.1).abs());

        // Already living on it: nothing to do, and importantly nothing that
        // keeps the agent walking in circles round the thing it wanted
        if paces <= Self::CAMPED_ON_IT {
            return None;
        }

        Some(Action::Move {
            target: (there.x, there.y, agent_position.2),
        })
    }

    /// How far a people will pick up and move for water they can count on.
    pub(in crate::analytics) const HOW_FAR_A_PEOPLE_WILL_MOVE: u32 = 60;

    /// What one person wants standing within reach of the camp before the
    /// ground counts as feeding them.
    ///
    /// Wild food regrows about four times slower than a settlement eats it, so
    /// a camp of any size strips its own ground and the number here is what
    /// "stripped" means. A nomad moves while there is still something to eat,
    /// because a nomad that waits until there is nothing has to walk on an
    /// empty stomach.
    ///
    /// The first cut of this was 25 a head, which is about what a person
    /// eats in a season and reads as the right number until you notice that
    /// no ground anywhere in the world carries that much for a grown
    /// settlement. It fired every tick of every life. Over eight worlds
    /// foraging fell forty per cent, the food standing on the map went up
    /// four and a half times because nobody was eating it, the camp did not
    /// end up any further from where it started, and it cost about twelve
    /// people.
    pub(in crate::analytics) const WHAT_A_CAMP_WANTS_STANDING: u32 = 4;

    /// And how much better somewhere else has to be before it is worth
    /// picking the camp up.
    ///
    /// This is the half that stops the walking. An absolute standard for good
    /// ground is a standard nowhere meets, so a camp held to one walks for
    /// ever; a camp that moves because somewhere is three times better stops
    /// the moment it gets there, because it is now standing on the best ground
    /// it knows of.
    pub(in crate::analytics) const WORTH_PICKING_THE_CAMP_UP_FOR: u32 = 3;

    /// Moving camp, for a people that has no other way of making food happen.
    ///
    /// "Until there is a method of producing food through farming, the agents
    /// should likely stick to a nomadic way of life."
    ///
    /// This is the Sustenance answer for anybody who cannot farm: you cannot
    /// make this ground carry more, so you go where the ground already does.
    /// An agent that has worked farming out does not do this - a field is a
    /// reason to stay, and the whole of what settling down is.
    ///
    /// It is not the same thing as `migration_action`, which fires on an agent
    /// that has already been going hungry for a hundred and twenty ticks. This
    /// fires while there is still food here, on the strength of there not
    /// being much of it, which is the difference between moving camp and
    /// fleeing.
    pub(in crate::analytics) fn moving_on(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> Option<Action> {
        use crate::agents::practices::Practice;

        // A farmer stays. So does anybody standing beside a field with
        // something in it, farmer or not: whatever is growing there is a
        // better answer than a fortnight's walk.
        if agent.practices.is_established(Practice::Farming) {
            return None;
        }

        if self.crop_standing_on_fields_within(agent_position, Self::FIELD_WALK_RADIUS) > 0 {
            return None;
        }

        // Enough hands here to strip the place, and enough standing to feed
        // them. Both are counted within the distance somebody actually walks
        // to forage.
        let mouths = self.how_many_camped_within(agent_position, Self::FORAGE_RADIUS);
        let standing = self.edible_standing_within(agent_position, Self::FORAGE_RADIUS);

        if standing >= mouths * Self::WHAT_A_CAMP_WANTS_STANDING {
            return None;
        }

        // Somewhere better, far enough off to be a move rather than a stroll.
        // The best ground within the distance a people will shift for, not the
        // nearest: this is a decision about where to spend a season.
        let here = crate::world::Position::new(agent_position.0, agent_position.1);

        let (there, carrying) = self
            .world
            .resources
            .iter()
            .filter(|resource| resource.amount > 0)
            .filter(|resource| Self::edible_item_for(resource.resource_type).is_some())
            .map(|resource| (resource.position, here.distance_to(&resource.position), resource.amount))
            .filter(|(_, distance, _)| {
                *distance >= Self::FAR_ENOUGH_TO_BE_WORTH_THE_WALK as u32
                    && *distance <= Self::HOW_FAR_A_PEOPLE_WILL_MOVE
            })
            .max_by_key(|(_, _, amount)| *amount)
            .map(|(where_it_is, _, amount)| (where_it_is, amount))?;

        // And it has to be worth the walk. Without this the camp sets out for
        // whatever is furthest, arrives, finds the same thin ground, and sets
        // out again: it walks for ever and forages a great deal less than a
        // people that stayed put.
        if carrying < standing.max(1) * Self::WORTH_PICKING_THE_CAMP_UP_FOR {
            return None;
        }

        Some(Action::Move {
            target: (there.x, there.y, agent_position.2),
        })
    }

    /// How many people are living within reach of this spot
    pub(in crate::analytics) fn how_many_camped_within(&self, position: (i32, i32, i32), radius: u32) -> u32 {
        let reach = radius as i32;

        self.population
            .agents
            .iter()
            .filter(|agent| agent.state.is_alive)
            .filter(|agent| {
                (agent.state.position.0 - position.0).abs() <= reach
                    && (agent.state.position.1 - position.1).abs() <= reach
            })
            .count() as u32
    }

    /// How much there is to eat standing within reach of this spot
    pub(in crate::analytics) fn edible_standing_within(&self, position: (i32, i32, i32), radius: u32) -> u32 {
        let here = crate::world::Position::new(position.0, position.1);

        self.world
            .resources
            .iter()
            .filter(|resource| Self::edible_item_for(resource.resource_type).is_some())
            .filter(|resource| here.distance_to(&resource.position) <= radius)
            .map(|resource| resource.amount)
            .sum()
    }

    /// And how much of that is standing on ground somebody has broken
    pub(in crate::analytics) fn crop_standing_on_fields_within(&self, position: (i32, i32, i32), radius: u32) -> u32 {
        let here = crate::world::Position::new(position.0, position.1);

        self.world
            .resources
            .iter()
            .filter(|resource| here.distance_to(&resource.position) <= radius)
            .filter(|resource| {
                self.world
                    .grid
                    .get_tile(&resource.position)
                    .map(|tile| tile.terrain.is_cultivated())
                    .unwrap_or(false)
            })
            .map(|resource| resource.amount)
            .sum()
    }

    pub(in crate::analytics) fn migration_action(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> Option<Action> {
        use crate::core::memory::SpatialMemoryType;

        // What this country has failed to give him.
        //
        // Hunger was the only thing here, and thirst kills a man three times
        // faster than hunger does. A settlement whose springs had gone dry
        // and whose hedgerows were full had no reason anywhere in this model
        // to pick up and leave, and did not: measured, eight of twenty-one
        // water sources drawn to nothing and a people still standing over
        // them at the end of the world. See ISSUES_FOUND #53.
        let going_without = Self::WHAT_A_COUNTRY_HAS_TO_PROVIDE
            .into_iter()
            .find(|drive| {
                agent
                    .drives
                    .get(*drive)
                    .map(|it| it.denied_ticks())
                    .unwrap_or(0)
                    >= Self::HUNGRY_ENOUGH_TO_LEAVE
            });

        let Some(going_without) = going_without else {
            return None;
        };

        // And what he would be walking towards. A man leaving for want of
        // water is not looking for a berry bush.
        let worth_walking_to = std::mem::discriminant(&match going_without {
            DriveType::Thirst => SpatialMemoryType::Water,
            _ => SpatialMemoryType::Food,
        });

        let far_off = |candidate: &(i32, i32, i32)| {
            (candidate.0 - agent_position.0)
                .abs()
                .max((candidate.1 - agent_position.1).abs())
        };

        // Somewhere it remembers food that is not on this doorstep. Anything
        // near enough to walk to in the ordinary way has already been tried by
        // the code above, and found wanting.
        //
        // Somewhere it remembers food that is not on this doorstep. Anything
        // near enough to walk to in the ordinary way has already been tried by
        // the code above, and found wanting.
        //
        // Which of them is decided by distance, which is unsatisfying and is
        // staying that way for now. A memory carries how much was standing
        // there, and choosing the richest instead - with and without weighing
        // it by how long ago he saw it - produced a rare world in which a
        // settlement refused for want of water 3,092, 851 and 13,004 times
        // against a worst case of seven, in three arms of thirty-two. The
        // reporting this belongs to is worth having on its own; this half
        // wants its own investigation and its own arm. See ISSUES_FOUND #68.
        let remembered = agent
            .memory
            .spatial_memories
            .iter()
            .filter(|memory| std::mem::discriminant(&memory.memory_type) == worth_walking_to)
            .map(|memory| (memory.position.0, memory.position.1, agent_position.2))
            .filter(|candidate| far_off(candidate) >= Self::FAR_ENOUGH_TO_BE_WORTH_THE_WALK)
            .max_by_key(far_off);

        if let Some(target) = remembered {
            return Some(Action::Move { target });
        }

        // Nothing remembered worth the walk: pick a bearing and hold it. The
        // bearing comes from the agent rather than the tick, so somebody who
        // sets out keeps going the same way instead of milling about, and two
        // people leaving the same place do not necessarily leave together.
        let bearings = [
            (1, 0),
            (0, 1),
            (-1, 0),
            (0, -1),
            (1, 1),
            (-1, 1),
            (1, -1),
            (-1, -1),
        ];
        let (dx, dy) = bearings[(agent.id.as_u128() % bearings.len() as u128) as usize];

        let target = (
            (agent_position.0 + dx * Self::FAR_ENOUGH_TO_BE_WORTH_THE_WALK)
                .clamp(0, self.world.grid.width as i32 - 1),
            (agent_position.1 + dy * Self::FAR_ENOUGH_TO_BE_WORTH_THE_WALK)
                .clamp(0, self.world.grid.height as i32 - 1),
            agent_position.2,
        );

        // Already hard against that edge: there is nowhere further this way
        if target.0 == agent_position.0 && target.1 == agent_position.1 {
            return None;
        }

        Some(Action::Move { target })
    }

    pub(in crate::analytics) fn search_leg(
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
        current_tick: u32,
    ) -> Action {
        const SEARCH_LEG_TICKS: u32 = 300;
        const SEARCH_LEG_DISTANCE: i32 = 12;

        let directions = [
            (1, 0),
            (-1, 0),
            (0, 1),
            (0, -1),
            (1, 1),
            (1, -1),
            (-1, 1),
            (-1, -1),
        ];

        let leg = (current_tick / SEARCH_LEG_TICKS) as u64;
        let seed = (agent.id.as_u128() as u64) ^ leg.wrapping_mul(0x9E37_79B9_7F4A_7C15);
        let (dx, dy) = directions[(seed % directions.len() as u64) as usize];

        Action::Move {
            target: (
                agent_position.0 + dx * SEARCH_LEG_DISTANCE,
                agent_position.1 + dy * SEARCH_LEG_DISTANCE,
                agent_position.2,
            ),
        }
    }

    /// Where the camp is, from where this agent is standing.
    ///
    /// There is no settlement object in this model - see ISSUES_FOUND #11 -
    /// so a camp is the nearest roof, and failing that the middle of whatever
    /// knot of people the agent is standing in. Both are rough and both are
    /// good enough to answer "is this plant near where I live".
    pub(in crate::analytics) fn where_the_camp_is(&self, position: (i32, i32, i32)) -> Option<crate::world::Position> {
        use crate::world::Position;

        // The people first, and the roof only when there are not enough of
        // them about to make a camp. `nearest_shelter_from` searches out from
        // wherever the agent is standing, so for a man twenty tiles out on the
        // moor it answers "the nearest cave to the moor", which is not his
        // home and is exactly the wrong answer to what this asks.
        let reach = Self::FORAGE_RADIUS as i32;

        let neighbours: Vec<(i32, i32)> = self
            .population
            .agents
            .iter()
            .filter(|agent| agent.state.is_alive)
            .map(|agent| (agent.state.position.0, agent.state.position.1))
            .filter(|(x, y)| {
                (x - position.0).abs() <= reach && (y - position.1).abs() <= reach
            })
            .collect();

        if (neighbours.len() as u32) >= Self::ENOUGH_PEOPLE_TO_BE_A_CAMP {
            return Some(Position::new(
                neighbours.iter().map(|(x, _)| x).sum::<i32>() / neighbours.len() as i32,
                neighbours.iter().map(|(_, y)| y).sum::<i32>() / neighbours.len() as i32,
            ));
        }

        self.nearest_shelter_from(position)
    }
}
