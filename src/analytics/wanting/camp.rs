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

/// A move of camp somebody has decided on, waiting on them setting off.
///
/// The decision layer only proposes: `moving_on` and
/// `go_and_live_where_it_is` take `&self`. Moving a camp moves other people's
/// hearths as well as the decider's, so it is carried out by the turn, once
/// the decider has actually set off (`Simulation::the_camp_moves`).
#[derive(Debug, Clone, PartialEq)]
pub struct CampMove {
    /// Who decided it.
    pub who: uuid::Uuid,
    /// Where the camp goes.
    pub to: (i32, i32),
    /// How many people the ground there will feed, counting the decider; or
    /// `None` for a move to something that does not run out, such as water.
    pub feeds: Option<u32>,
}

impl Simulation {
    /// How far apart two hearths can be and still be one camp, in cells.
    ///
    /// Twice the distance from home at which somebody walks back in the
    /// evening, so that everybody who sleeps on the edge of a camp still
    /// counts the far edge as theirs.
    pub(in crate::analytics) const THE_REACH_OF_A_CAMP: i32 = 2 * Self::NEAR_ENOUGH_TO_SLEEP_WITH_THE_OTHERS;

    /// How long a camp that has just moved stays before it will move again.
    ///
    /// A week: long enough for the people who went with it to get there and
    /// the place to be tried, and short of the few weeks a foraging camp
    /// usually stays.
    pub(in crate::analytics) const A_CAMP_STAYS_AT_LEAST: u32 = 7 * crate::environment::seasons::TICKS_PER_DAY;

    /// Where this one lives: their hearth, or before their first night,
    /// where they stand.
    pub(in crate::analytics) fn where_this_one_lives(agent: &crate::agents::Agent) -> (i32, i32) {
        agent
            .hearth
            .unwrap_or((agent.state.position.0, agent.state.position.1))
    }

    /// The grown people who live within reach of a place, by index.
    pub(in crate::analytics) fn who_lives_near(&self, at: (i32, i32)) -> Vec<usize> {
        self.population
            .agents
            .iter()
            .enumerate()
            .filter(|(_, agent)| agent.state.is_alive && agent.state.life_stage.can_reproduce())
            .filter(|(_, agent)| {
                let lives = Self::where_this_one_lives(agent);
                (lives.0 - at.0).abs().max((lives.1 - at.1).abs()) <= Self::THE_REACH_OF_A_CAMP
            })
            .map(|(index, _)| index)
            .collect()
    }

    /// Where this one sleeps tonight: in the middle of the people who live
    /// where they live, or under a finished roof among them.
    ///
    /// **Each person's own, not the world's.** This was one answer for
    /// everybody, the middle of every grown person on the map, so there could
    /// only ever be one camp: anybody who went off for water or better ground
    /// was walked back to the others that evening, however far it was. Now
    /// the middle is found from where this one lives, by stepping to the
    /// middle of the hearths in reach of it until it settles, so two knots of
    /// people a few kilometres apart are two camps, and two that drift within
    /// reach of each other become one (#314).
    ///
    /// A child not yet grown sleeps where a parent does.
    pub(in crate::analytics) fn where_this_one_sleeps(&self, agent_index: usize) -> Option<(i32, i32)> {
        let agent = self.population.agents.get(agent_index)?;

        if !agent.state.life_stage.can_reproduce() {
            let parent = self.population.agents.iter().position(|them| {
                them.state.is_alive
                    && them.state.life_stage.can_reproduce()
                    && agent.parent_ids.contains(&them.id)
            });
            if let Some(parent) = parent {
                return self.where_this_one_sleeps(parent);
            }
        }

        let mut at = self.the_middle_of_the_camp_at(Self::where_this_one_lives(agent));

        // **Nobody lives alone by accident.** Somebody with no other grown
        // person living within reach, nothing standing in a field to keep
        // them, and no fresh move of their own to wait out, goes to the
        // nearest camp there is. Without this, whoever was off on their own
        // the first night a hearth was kept made a camp of one there and
        // stayed in it for good: one sat 115 cells from the others for
        // months on seed 0 (#314).
        let alone = self
            .who_lives_near(at)
            .iter()
            .all(|&index| index == agent_index);
        if alone && !self.tied_to_this_ground(agent) && !self.moved_lately(agent) {
            if let Some(camp) = self.the_nearest_camp_to(at, agent_index) {
                at = camp;
            }
        }

        // **Where the camp's people are tonight, not where they slept last
        // night.** The hearths say which camp this one belongs to; where it
        // sleeps tonight is the middle of where its people actually are, so
        // that a camp drifts towards the ground it is foraging, as it always
        // did when home was the middle of everybody. Taken from the hearths
        // alone, it was a fixed point - home was where the hearths were and
        // the hearths were set to home - and a camp that started by the
        // longhouse slept there for ever, walking back every evening to
        // ground it had stripped: births on four fresh worlds fell from 30
        // to 12 in two years (#314). Everybody who lives there counts, as
        // everybody did, so that with one camp in the world this is exactly
        // the old answer. The one exception is a camp that has just moved:
        // its people are still on their way, the middle of them is out of
        // reach of where they now live, and home is the new place.
        let here_tonight: Vec<(i32, i32)> = self
            .who_lives_near(at)
            .into_iter()
            .map(|index| {
                let position = self.population.agents[index].state.position;
                (position.0, position.1)
            })
            .collect();
        if !here_tonight.is_empty() {
            let n = here_tonight.len() as i32;
            let middle = (
                here_tonight.iter().map(|p| p.0).sum::<i32>() / n,
                here_tonight.iter().map(|p| p.1).sum::<i32>() / n,
            );
            if (middle.0 - at.0).abs().max((middle.1 - at.1).abs()) <= Self::THE_REACH_OF_A_CAMP {
                at = middle;
            }
        }

        let roof = self
            .world
            .buildings
            .iter()
            .filter(|roof| roof.is_completed())
            .map(|roof| (roof.position.x, roof.position.y))
            .filter(|roof| {
                (roof.0 - at.0).abs().max((roof.1 - at.1).abs()) <= Self::FORAGE_RADIUS as i32
            })
            .min_by_key(|roof| (roof.0 - at.0).abs() + (roof.1 - at.1).abs());
        Some(roof.unwrap_or(at))
    }

    /// The middle of the people who live around a place, found by stepping to
    /// the middle of the hearths in reach until it settles.
    pub(in crate::analytics) fn the_middle_of_the_camp_at(&self, start: (i32, i32)) -> (i32, i32) {
        let mut at = start;
        for _ in 0..4 {
            let near = self.who_lives_near(at);
            if near.is_empty() {
                break;
            }
            let n = near.len() as i32;
            let (sx, sy) = near.iter().fold((0, 0), |(sx, sy), &index| {
                let lives = Self::where_this_one_lives(&self.population.agents[index]);
                (sx + lives.0, sy + lives.1)
            });
            let middle = (sx / n, sy / n);
            if middle == at {
                break;
            }
            at = middle;
        }
        at
    }

    /// The middle of the nearest camp to a place, within the distance a
    /// people will move: the nearest grown person, other than `but`, who lives
    /// among enough others to be a camp (`ENOUGH_PEOPLE_TO_BE_A_CAMP`).
    pub(in crate::analytics) fn the_nearest_camp_to(&self, from: (i32, i32), but: usize) -> Option<(i32, i32)> {
        let distance = |at: (i32, i32)| (at.0 - from.0).abs().max((at.1 - from.1).abs());
        let mut people: Vec<(i32, (i32, i32))> = self
            .population
            .agents
            .iter()
            .enumerate()
            .filter(|(index, agent)| {
                *index != but && agent.state.is_alive && agent.state.life_stage.can_reproduce()
            })
            .map(|(_, agent)| {
                let lives = Self::where_this_one_lives(agent);
                (distance(lives), lives)
            })
            .filter(|(paces, _)| *paces as u32 <= Self::HOW_FAR_A_PEOPLE_WILL_MOVE)
            .collect();
        people.sort();
        people.into_iter().find_map(|(_, lives)| {
            let middle = self.the_middle_of_the_camp_at(lives);
            let living_there = self
                .who_lives_near(middle)
                .into_iter()
                .filter(|&index| index != but)
                .count() as u32;
            (living_there >= Self::ENOUGH_PEOPLE_TO_BE_A_CAMP).then_some(middle)
        })
    }

    /// Somebody lying down for the night at home: their hearth is the camp
    /// they are sleeping in, wherever it has drifted to. Not a move, so it
    /// does not count as one (`hearth_moved_at`).
    pub(in crate::analytics) fn settle_the_hearth(&mut self, agent_index: usize) {
        let Some(home) = self.where_this_one_sleeps(agent_index) else {
            return;
        };
        let agent = &mut self.population.agents[agent_index];
        let here = agent.state.position;
        if (home.0 - here.0).abs().max((home.1 - here.1).abs()) <= Self::NEAR_ENOUGH_TO_SLEEP_WITH_THE_OTHERS {
            agent.hearth = Some(home);
        }
    }

    /// Whether this one has a reason to stay where they live when the camp
    /// moves: a crop standing on a field within a walk of it.
    pub(in crate::analytics) fn tied_to_this_ground(&self, agent: &crate::agents::Agent) -> bool {
        let lives = Self::where_this_one_lives(agent);
        self.crop_standing_on_fields_within((lives.0, lives.1, 0), Self::FIELD_WALK_RADIUS) > 0
    }

    /// Whether there is food in a pit within a forage of a place.
    pub(in crate::analytics) fn food_in_store_near(&self, at: (i32, i32)) -> bool {
        let reach = Self::FORAGE_RADIUS as i32;
        self.world.pits.iter().any(|pit| {
            (pit.where_it_is.x - at.0).abs().max((pit.where_it_is.y - at.1).abs()) <= reach
                && pit.how_much_is_in_it() > 0
        })
    }

    /// Whether this one's camp moved too lately to move again.
    pub(in crate::analytics) fn moved_lately(&self, agent: &crate::agents::Agent) -> bool {
        agent.hearth.is_some()
            && agent.hearth_moved_at > 0
            && self.current_turn.saturating_sub(agent.hearth_moved_at) < Self::A_CAMP_STAYS_AT_LEAST
    }

    /// Note a move of camp for the turn to carry out if the person sets off.
    pub(in crate::analytics) fn propose_moving_camp(&self, agent: &crate::agents::Agent, to: (i32, i32), feeds: Option<u32>) {
        *self.camp_move_proposed.borrow_mut() = Some(CampMove { who: agent.id, to, feeds });
    }

    /// The camp moves: the one who decided it, and as many of the people who
    /// live with them as the new ground will feed.
    ///
    /// **Who goes is nobody's ruling.** Everybody who lives within reach of
    /// the decider's hearth goes, unless they have a crop standing on a field
    /// by it, most trusted by the decider first, until the new ground is full
    /// (`CampMove::feeds`). The rest stay where they are, where there are
    /// fewer mouths now to feed. That is how a camp that has outgrown its
    /// ground becomes two, and a farmer stays by a field while the foragers go
    /// on. Children not yet grown go with a parent who goes. See #314.
    pub(in crate::analytics) fn the_camp_moves(&mut self, decided: CampMove) {
        let Some(decider) = self.population.agents.iter().position(|a| a.id == decided.who) else {
            return;
        };
        let from = Self::where_this_one_lives(&self.population.agents[decider]);
        let now = self.current_turn.max(1);

        let mut mates: Vec<usize> = self
            .who_lives_near(from)
            .into_iter()
            .filter(|&index| index != decider)
            .filter(|&index| !self.tied_to_this_ground(&self.population.agents[index]))
            .collect();
        {
            let them = &self.population.agents;
            let trust = |index: usize| them[decider].how_far_i_trust(them[index].id, &them[index].traits);
            mates.sort_by(|a, b| trust(*b).partial_cmp(&trust(*a)).unwrap_or(std::cmp::Ordering::Equal));
        }
        let room = decided.feeds.map(|n| n.saturating_sub(1) as usize).unwrap_or(usize::MAX);
        let stayed = mates.len().saturating_sub(room);
        mates.truncate(room);

        let mut going: Vec<uuid::Uuid> = vec![self.population.agents[decider].id];
        going.extend(mates.iter().map(|&index| self.population.agents[index].id));

        for agent in self.population.agents.iter_mut().filter(|a| a.state.is_alive) {
            let grown = agent.state.life_stage.can_reproduce();
            let goes = going.contains(&agent.id)
                || (!grown && agent.parent_ids.iter().any(|parent| going.contains(parent)));
            if goes {
                agent.hearth = Some(decided.to);
                agent.hearth_moved_at = now;
            }
        }

        let stats = &mut self.population.stats.how_it_went;
        *stats.entry("camp moved".to_string()).or_insert(0) += 1;
        *stats.entry("went with the camp".to_string()).or_insert(0) += mates.len() as u64;
        if stayed > 0 {
            *stats.entry("camp split: some stayed".to_string()).or_insert(0) += 1;
        }
    }

    /// The need this agent keeps having and keeps not getting.
    ///
    /// `denied_turns` counts how long a drive has gone unanswered, and until
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
        // more than half its fertility inside ten thousand turns.
        //
        // Ranging for food and settling by water is the division the land
        // itself makes.
        [DriveType::Thirst]
            .into_iter()
            .filter(|need| {
                agent
                    .drives
                    .get(*need)
                    .map(|drive| drive.denied_turns() >= Self::ASKED_FOR_IT_ONCE_TOO_OFTEN)
                    .unwrap_or(false)
            })
            .max_by_key(|need| {
                agent
                    .drives
                    .get(*need)
                    .map(|drive| drive.denied_turns())
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
        if self.current_turn % crate::environment::seasons::TICKS_PER_DAY != 0 {
            return None;
        }

        let short_of = Self::what_i_keep_going_short_of(agent)?;

        let wanted = match short_of {
            DriveType::Thirst => ResourceType::Water,
            _ => ResourceType::Food,
        };

        // The nearest place that answers it from a spot, however far off -
        // this is a decision about where to live, so the ordinary foraging
        // radius does not apply - unless the spot is already on it.
        let answer_from = |from: (i32, i32, i32)| {
            let there = self.nearest_resource_within(agent, from, Self::HOW_FAR_A_PEOPLE_WILL_MOVE, |resource| {
                resource.resource_type == wanted
                    || (wanted == ResourceType::Food
                        && Self::edible_item_for(resource.resource_type).is_some())
            })?;
            let paces = (there.x - from.0).abs().max((there.y - from.1).abs());
            (paces > Self::CAMPED_ON_IT).then_some(there)
        };

        // **Two questions, from two places** (#314). Whether the camp lives on
        // the water is asked from where this one lives, and if it does not,
        // and has not just moved, the camp goes, everybody who lives there
        // with it: water does not run out.
        let lives = Self::where_this_one_lives(agent);
        if !self.moved_lately(agent) {
            if let Some(there) = answer_from((lives.0, lives.1, agent_position.2)) {
                self.propose_moving_camp(agent, (there.x, there.y), None);
                return Some(Action::Move {
                    target: (there.x, there.y, agent_position.2),
                });
            }
        }

        // And whether this one, wherever they are, still has to go and get
        // it is asked from where they stand, as it always was. Asking only
        // from the hearth left somebody thirsty for days, out on the far
        // side of the country from a camp that lived by water, with no walk
        // to water at all, and days of thirst shut the gate for a child.
        let there = answer_from(agent_position)?;
        Some(Action::Move {
            target: (there.x, there.y, agent_position.2),
        })
    }

    /// How far a people will pick up and move for water they can count on.
    ///
    /// Six hundred metres when a person walked a cell a turn; six kilometres
    /// now that they walk at five kilometres an hour - see `world::pace`.
    pub(in crate::analytics) const HOW_FAR_A_PEOPLE_WILL_MOVE: u32 = 60 * crate::world::pace::HOW_MUCH_FURTHER_IS_WORTH_WALKING;

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
    /// settlement. It fired every turn of every life. Over eight worlds
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
    /// that has already been going hungry for a hundred and twenty turns. This
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

        // **A move of camp, or a day's trip.** Whether the camp itself goes
        // is a question about where this one lives: not straight after a
        // move, and not away from food in store, because a people with
        // something in a pit by the camp eat out of it rather than walk off
        // and leave it. Without the store rule, every winter, when farming
        // lapsed for want of a crop to show for it, the camp went back to
        // ranging a whole camp at a time, up to once a week, away from the
        // pits holding the winter (#314).
        //
        // Otherwise this one still goes for the day, judged from where they
        // stand, as everybody always did: before there was a hearth this was
        // the long foraging trip, out to the best ground they knew and back
        // to the others at night, and it fed people.
        let lives = Self::where_this_one_lives(agent);
        if !self.moved_lately(agent) && !self.food_in_store_near(lives) {
            if let Some((there, carrying)) = self.better_ground_from((lives.0, lives.1, agent_position.2), agent) {
                // As many go as the ground there will feed, by the same
                // measure that says this ground is stripped.
                self.propose_moving_camp(
                    agent,
                    (there.x, there.y),
                    Some((carrying / Self::WHAT_A_CAMP_WANTS_STANDING).max(1)),
                );
                return Some(Action::Move {
                    target: (there.x, there.y, agent_position.2),
                });
            }
        }

        let (there, _) = self.better_ground_from(agent_position, agent)?;
        Some(Action::Move {
            target: (there.x, there.y, agent_position.2),
        })
    }

    /// Ground clearly better than this, for a people that cannot make food
    /// happen where they are: `None` while there is enough standing for the
    /// mouths within a forage, or nowhere known is worth the walk. With what
    /// the ground there carries.
    pub(in crate::analytics) fn better_ground_from(
        &self,
        agent_position: (i32, i32, i32),
        agent: &crate::agents::Agent,
    ) -> Option<(crate::world::Position, u32)> {

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
            .nodes_known_to(agent, here, Self::HOW_FAR_A_PEOPLE_WILL_MOVE)
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

        Some((there, carrying))
    }

    /// How much better somewhere has to be for a farmer before the camp moves
    /// to it.
    ///
    /// Half as good again: less than the three times better a forager wants
    /// (`WORTH_PICKING_THE_CAMP_UP_FOR`), because a forager moves to a season's
    /// food and a farmer to the ground he will work for years, and good
    /// ploughland is rarer than a full hedgerow.
    pub(in crate::analytics) const FARMLAND_WORTH_MOVING_FOR: f32 = 1.5;

    /// How many places a farmer weighs when thinking about where to live.
    pub(in crate::analytics) const PLACES_A_FARMER_WEIGHS: usize = 12;

    /// How long in a day somebody gives to thinking about where to live, in
    /// planning periods, each at their own time of day.
    pub(in crate::analytics) const A_SPELL_THINKING_ABOUT_WHERE_TO_LIVE: u32 = 6;

    /// Whether this is one of this one's planning periods for thinking about
    /// where to live. It walks a few thousand tiles, so it is a question for
    /// a few half-hours a day, not every turn.
    pub(in crate::analytics) fn time_to_think_about_where_to_live(&self, agent: &crate::agents::Agent) -> bool {
        use crate::environment::seasons::{PLANNING_PERIODS_PER_DAY, TICKS_BETWEEN_PLANS};
        let period = (self.current_turn / TICKS_BETWEEN_PLANS) % PLANNING_PERIODS_PER_DAY;
        let mine = (agent.id.as_u128() % PLANNING_PERIODS_PER_DAY as u128) as u32;
        (period + PLANNING_PERIODS_PER_DAY - mine) % PLANNING_PERIODS_PER_DAY
            < Self::A_SPELL_THINKING_ABOUT_WHERE_TO_LIVE
    }

    /// What the ground within a field's walk of a place is worth to a farmer:
    /// every tile a plough will take, at what it carries.
    ///
    /// Read off the ground, as `tilling_soil` reads wild ground: what grows on
    /// it wild is what it is, and anybody standing there can see it.
    pub(in crate::analytics) fn what_this_ground_is_worth_to_a_farmer(&self, at: (i32, i32)) -> f32 {
        use crate::world::Position;

        let reach = Self::FIELD_WALK_RADIUS as i32;
        let mut worth = 0.0;
        for dx in -reach..=reach {
            for dy in -reach..=reach {
                let tile_at = Position::new(at.0 + dx, at.1 + dy);
                let Some(tile) = self.world.grid.get_tile(&tile_at) else {
                    continue;
                };
                if tile.terrain.can_be_tilled() || tile.terrain.is_cultivated() {
                    worth += self.world.grid.how_good_the_ground_is(&tile_at);
                }
            }
        }
        worth
    }

    /// Moving camp to better ploughland, for a people that has taken up
    /// farming.
    ///
    /// "Once farming is discovered, the agents should naturally shift towards
    /// land which supports farming." Before farming people range for food
    /// (`moving_on`), and a farmer never did that; what a farmer had instead
    /// was nothing, so a people stayed wherever they happened to be when
    /// somebody first saw seed come up. Now a farmer weighs the ground around
    /// where they live against places they know wild grain or wild pulses
    /// grow, which is ground a plough will take, and takes the camp to the
    /// best of them when it is clearly better. Whoever lives with them goes
    /// too, unless they have a crop standing (`the_camp_moves`). See #314.
    pub(in crate::analytics) fn moving_to_farmland(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> Option<Action> {
        use crate::agents::practices::Practice;
        use crate::world::{Position, ResourceType};

        if !agent.practices.is_established(Practice::Farming)
            || !self.time_to_think_about_where_to_live(agent)
            || self.moved_lately(agent)
        {
            return None;
        }

        let lives = Self::where_this_one_lives(agent);
        let home = Position::new(lives.0, lives.1);
        let here = self.what_this_ground_is_worth_to_a_farmer(lives);

        let mut places: Vec<Position> = agent
            .exploration_knowledge
            .known_resources
            .iter()
            .filter(|(_, kind)| matches!(kind, ResourceType::Grain | ResourceType::Legumes))
            .map(|(at, _)| *at)
            .filter(|at| {
                let distance = home.distance_to(at);
                distance >= Self::FAR_ENOUGH_TO_BE_WORTH_THE_WALK as u32
                    && distance <= Self::HOW_FAR_A_PEOPLE_WILL_MOVE
            })
            .collect();
        places.sort_by_key(|at| home.distance_to(at));
        places.truncate(Self::PLACES_A_FARMER_WEIGHS);

        let (there, worth) = places
            .into_iter()
            .map(|at| (at, self.what_this_ground_is_worth_to_a_farmer((at.x, at.y))))
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))?;

        if worth < here.max(1.0) * Self::FARMLAND_WORTH_MOVING_FOR {
            return None;
        }

        self.propose_moving_camp(agent, (there.x, there.y), None);
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
            .nodes_near(here, radius)
            .filter(|resource| Self::edible_item_for(resource.resource_type).is_some())
            .filter(|resource| here.distance_to(&resource.position) <= radius)
            .map(|resource| resource.amount)
            .sum()
    }

    /// And how much of that is standing on ground somebody has broken
    pub(in crate::analytics) fn crop_standing_on_fields_within(&self, position: (i32, i32, i32), radius: u32) -> u32 {
        let here = crate::world::Position::new(position.0, position.1);

        self.world
            .nodes_near(here, radius)
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
                    .map(|it| it.denied_turns())
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
        // bearing comes from the agent rather than the turn, so somebody who
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
        current_turn: u32,
        map: (usize, usize),
    ) -> Action {
        const SEARCH_LEG_TURNS: u32 = 300;
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

        let leg = (current_turn / SEARCH_LEG_TURNS) as u64;
        let seed = (agent.id.as_u128() as u64) ^ leg.wrapping_mul(0x9E37_79B9_7F4A_7C15);
        let first = (seed % directions.len() as u64) as usize;

        // **Not a bearing that goes nowhere.** The leg went twelve paces on
        // its bearing wherever that landed, and on a bearing that held for
        // 300 turns somebody near the edge was sent past it for the whole of
        // them: they walked to the last tile, and every step after that was a
        // route search over the whole map for somewhere that is not on it
        // (#294). Where it ends is now kept on the map, on ground, by
        // `where_a_walk_can_end` along with every other walk; and where this
        // bearing has nowhere left to go - somebody standing hard against
        // that edge already - it turns to the next bearing round that has,
        // so somebody desperate still strikes out rather than standing still.
        let (wide, high) = (map.0 as i32, map.1 as i32);
        let leg = |(dx, dy): (i32, i32)| {
            (
                agent_position.0 + dx * SEARCH_LEG_DISTANCE,
                agent_position.1 + dy * SEARCH_LEG_DISTANCE,
                agent_position.2,
            )
        };
        let goes_somewhere = |target: &(i32, i32, i32)| {
            (target.0.clamp(0, wide - 1), target.1.clamp(0, high - 1))
                != (agent_position.0, agent_position.1)
        };
        let target = (0..directions.len())
            .map(|turned| leg(directions[(first + turned) % directions.len()]))
            .find(goes_somewhere)
            .unwrap_or(agent_position);

        Action::Move { target }
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
