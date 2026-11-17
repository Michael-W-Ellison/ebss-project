// src/analytics/wanting/ground.rs
//! Working the ground, before there is anything on it to take.
//!
//! Breaking ground, sowing, tending, muck, cuttings, and the tasting of a
//! plant nobody has tried - which is how anything gets onto this list at all.
//!
//! Part of the decision layer - see [`super`]. Nothing here does anything: it
//! answers what would be worth doing, and hands that answer back up the ladder.

use super::super::Simulation;
use crate::core::DriveType;
use crate::environment::Action;

impl Simulation {
    /// Everything an agent might put in the ground, by what a pack calls it.
    ///
    /// Seed is not a separate thing an agent carries: a handful of the grain
    /// in the pack is next year's field, which is exactly the choice a hungry
    /// people has to make.
    pub(in crate::analytics) fn what_can_be_sown() -> [(&'static str, crate::world::ResourceType, bool); 9] {
        use crate::world::ResourceType;

        // The flag is whether it is worth breaking ground for when the thing
        // driving you is hunger. Sprouted grain comes first because it is the
        // one thing in the list that is visibly already doing what a field is
        // for - a man holding it does not have to be told.
        //
        // Seed set aside comes before all of it: it was kept back for this
        // and for nothing else (#365). And first in the list is the name a
        // grain field's lesson is recorded under, so the lesson and the
        // choice of seed are the same lesson.
        //
        // Roots go in as a piece of the tuber, or the top of one put back
        // in the hole it came out of, which is how growing them began
        // (#384): "propagation from cuttings, tubers, and saved seed".
        [
            (crate::agents::Agent::SEED, ResourceType::Grain, true),
            ("sproutedgrain", ResourceType::Grain, true),
            ("grain", ResourceType::Grain, true),
            ("legumes", ResourceType::Legumes, true),
            ("roots", ResourceType::Roots, true),
            ("food", ResourceType::Food, true),
            ("flax", ResourceType::Flax, false),
            ("cotton", ResourceType::Cotton, false),
            ("herbs", ResourceType::Herbs, false),
        ]
    }

    /// What this agent puts in the ground, given what it is carrying and what
    /// it has come to think of each.
    ///
    /// Of the sowable things in the pack it picks the one its own record rates
    /// best - which for an agent that has never farmed is whichever comes
    /// first, and for one that has walked back to a field of berries three
    /// autumns running is emphatically not berries. An agent carrying nothing
    /// sowable puts in what it has been eating, and learns from that too.
    #[cfg(test)]
    pub(in crate::analytics) fn what_this_one_would_sow(
        agent: &crate::agents::Agent,
        how_good_the_ground_is: f32,
    ) -> crate::world::ResourceType {
        Self::what_this_one_would_sow_on(agent, how_good_the_ground_is, None)
            .unwrap_or(crate::world::ResourceType::Food)
    }

    /// What this one would sow on this day of the year, or nothing if
    /// nothing they would sow is in its time (#418): the same choice, with
    /// a staple put first by somebody who has one (#421), and, by somebody
    /// who keeps a planting calendar, only what is in its time. With no day,
    /// every crop is in its time.
    pub(in crate::analytics) fn what_this_one_would_sow_on(
        agent: &crate::agents::Agent,
        how_good_the_ground_is: f32,
        day_of_year: Option<u32>,
    ) -> Option<crate::world::ResourceType> {
        use crate::world::ResourceType;

        let in_its_time = |crop: ResourceType| match day_of_year {
            Some(day) if agent.knows_the_way("planting_calendar") => Self::is_it_the_time_to_sow(crop, day),
            _ => true,
        };
        // The crop somebody has settled on, by the kind it is (#421)
        let staple = agent.knows_the_way("staple").then(|| agent.my_staple_crop()).flatten().and_then(|called| {
            Self::what_can_be_sown()
                .into_iter()
                .find(|(name, _, _)| *name == called)
                .map(|(_, crop, _)| crop)
        });

        let mut best: Option<(ResourceType, f32)> = None;

        for (called, crop, feeds_anybody) in Self::what_can_be_sown() {
            // A field is broken to answer hunger, so what goes in it is
            // something a person can eat. The first cut of this let an agent
            // sow whatever was in the pack, and over eight worlds the people
            // put in flax and cotton and starved beside their own linen.
            if !feeds_anybody {
                continue;
            }

            if agent.how_many_i_have(called) == 0 || !in_its_time(crop) {
                continue;
            }

            let mut believed = agent
                .lessons
                .how_likely_to_try_this(&format!("sow:{called}"));

            // What has paid best goes in first, by somebody who has settled
            // on it (#421)
            if staple == Some(crop) {
                believed += Self::WHAT_A_STAPLE_IS_WORTH;
            }

            // And what the ground under this man's feet is worth, which he
            // can see: thin, pale, hungry-looking stuff that grew a poor
            // crop last time. On ground like that a pod row is the thing to
            // put in, and this is as close to a rotation as this model can
            // honestly get.
            //
            // It is a *reading*, not a plan. Nothing here connects a bean
            // sown this year to the wheat that will do better next year on
            // the same tile - the lessons are keyed on the crop and not on
            // the ground, so that chain cannot be learned. What an agent can
            // do is notice that poor ground repays beans, which is what a
            // farmer noticing his own field looks like from the outside and
            // is how most of this was actually found out.
            if crop.feeds_the_ground() && how_good_the_ground_is < Self::TOO_POOR_FOR_A_HUNGRY_CROP {
                believed += Self::WHAT_A_POD_ROW_IS_WORTH_ON_TIRED_GROUND;
            }

            if best.map(|(_, so_far)| believed > so_far).unwrap_or(true) {
                best = Some((crop, believed));
            }
        }

        match best {
            Some((crop, _)) => Some(crop),
            // An agent carrying nothing sowable puts in what it has been
            // eating, if that is in its time
            None => in_its_time(ResourceType::Food).then_some(ResourceType::Food),
        }
    }

    /// How much a staple counts for in choosing what to sow (#421): more
    /// than the pod row on tired ground, so a staple of grain still goes in
    /// on poor ground, and enough to beat a middling opinion.
    pub(in crate::analytics) const WHAT_A_STAPLE_IS_WORTH: f32 = 0.5;

    /// Whether this is the time to sow this crop, by a planting calendar
    /// (#418): before its season, or early enough in it that most of its
    /// growing is still to come. A crop grows only in its season, so one
    /// sown with less than half of it left ripens short, and a short crop
    /// pays less than went into it.
    pub(in crate::analytics) fn is_it_the_time_to_sow(crop: crate::world::ResourceType, day_of_year: u32) -> bool {
        let window = crop.bearing_window();
        match window.days_left(day_of_year) {
            None => true,
            Some(left) => left * 2 >= window.how_many_days(),
        }
    }

    /// How poor ground has to look before a man puts beans in it rather than
    /// wheat.
    ///
    /// Half of what the ground can hold. Above that a field is still worth
    /// cropping and a hungry people crops it; below it the crop that comes
    /// off is thin enough that a year of pods is the better bargain, which is
    /// exactly the trade a two-course rotation is.
    pub(in crate::analytics) const TOO_POOR_FOR_A_HUNGRY_CROP: f32 = 0.5;

    /// And how much that reading is worth against what he has learned.
    ///
    /// Enough to beat an ordinary opinion and not enough to beat a strong
    /// one: a man who has sown wheat on this ground three years running and
    /// carried it home each time still sows wheat.
    pub(in crate::analytics) const WHAT_A_POD_ROW_IS_WORTH_ON_TIRED_GROUND: f32 = 0.3;

    /// How much comes up off one tile's worth of seed.
    pub(in crate::analytics) const WHAT_A_MIDDEN_COMES_UP_IN: f32 = 8.0;

    /// How wet it has to be under a pack before grain in it starts moving.
    ///
    /// Set against `Soil::humidity`, which reads the country and the sky
    /// together: a wetland or a riverbank is wet enough standing still, a
    /// forest floor is on the line, and open plains only get there when it is
    /// actually raining. Dry ground under a clear sky never does.
    pub(in crate::analytics) const WET_ENOUGH_TO_START_IT: f32 = 0.7;

    /// And how readily it goes, per turn, at that wetness.
    ///
    /// Slow: a handful of grain that gets rained on does not come up the same
    /// afternoon. Over a wet season most of what a person is carrying will
    /// have started, which is the point - the seed spoils as food and becomes
    /// something else.
    pub(in crate::analytics) const HOW_READILY_GRAIN_TAKES: f32 = 0.01;

    /// Fields already broken within reach that are still worth sowing.
    ///
    /// A field worked down to exhaustion is not counted. It yields a quarter
    /// of ordinary ground, and while it counted as one of the
    /// `FIELDS_WANTED` a settlement that had worn its ground out saw its full
    /// tally of fields and broke no new ground, and starved beside them (#5).
    /// Left alone it rests back towards what it was, and counts again when
    /// it has climbed a rung.
    pub(in crate::analytics) fn fields_within(&self, position: (i32, i32, i32), radius: u32) -> usize {
        use crate::world::Position;

        let from = Position::new(position.0, position.1);

        let reach = radius as i32;
        let mut fields = 0;

        for dx in -reach..=reach {
            for dy in -reach..=reach {
                let candidate = Position::new(from.x + dx, from.y + dy);

                if from.distance_to(&candidate) > radius {
                    continue;
                }

                let cultivated = self
                    .world
                    .grid
                    .get_tile(&candidate)
                    .map(|tile| tile.terrain.is_cultivated())
                    .unwrap_or(false);
                let spent = self
                    .world
                    .grid
                    .field_at(&candidate)
                    .is_some_and(|field| field.grade == crate::world::soil::SoilGrade::Exhausted);
                if cultivated && !spent {
                    fields += 1;
                }
            }
        }

        fields
    }

    /// Somewhere nearby worth breaking: open grass with nothing growing on it
    pub(in crate::analytics) fn ground_to_break(&self, position: (i32, i32, i32)) -> Option<crate::world::Position> {
        use crate::world::Position;

        let from = Position::new(position.0, position.1);
        let radius = Self::FIELD_WALK_RADIUS as i32;

        // What is already growing, gathered once: asking the resource list per
        // candidate tile turns this into tens of thousands of comparisons per
        // agent per turn
        let occupied: std::collections::BTreeSet<(i32, i32)> = self
            .world
            .nodes_near(from, radius as u32)
            .map(|resource| (resource.position.x, resource.position.y))
            .collect();

        let mut best: Option<(Position, u32)> = None;

        for dx in -radius..=radius {
            for dy in -radius..=radius {
                let candidate = Position::new(from.x + dx, from.y + dy);

                if occupied.contains(&(candidate.x, candidate.y)) {
                    continue;
                }

                if !self.world.grid.is_valid_position(&candidate) {
                    continue;
                }

                // Open grass, or a field with nothing standing on it: a crop
                // ploughed in leaves ground that wants sowing again, and it
                // is nearer and already broken.
                let tillable = self
                    .world
                    .grid
                    .get_tile(&candidate)
                    .map(|tile| tile.terrain.can_be_tilled() || tile.terrain.is_cultivated())
                    .unwrap_or(false);

                if !tillable {
                    continue;
                }

                let distance = from.distance_to(&candidate);
                if best.map(|(_, d)| distance < d).unwrap_or(true) {
                    best = Some((candidate, distance));
                }
            }
        }

        best.map(|(position, _)| position)
    }

    /// Tipping the spoiled contents of a pack onto the ground.
    ///
    /// Nothing tells an agent to do this. It carries refuse it cannot eat, it
    /// is standing on ground it has broken, and now and again - out of
    /// curiosity, and more often once it has half a notion the thing works - it
    /// tips the basket out and sees what happens. What it sees is the ground
    /// getting richer, which is worth something; what it works out over several
    /// seasons is whether that was worth the carrying.
    ///
    /// The practice spreads by being seen, and it is dropped by agents who try
    /// it half a dozen times on ground where it does nothing.
    pub(in crate::analytics) fn muck_action(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> Option<Action> {
        use crate::agents::practices::Practice;
        use crate::world::Position;
        use rand::Rng;

        // Composting is Stage 2 of the agricultural path, and comes with
        // gardens: a people with no field has nowhere to tip it
        if !agent.knows_how_to_do("SpreadMuck") {
            return None;
        }

        // Nothing to tip out
        let carrying_refuse = agent
            .inventory
            .get_all_items()
            .values()
            .filter(|item| item.quantity > 0)
            .any(|item| {
                item.food_data
                    .as_ref()
                    .map(|food| food.is_rotting() || food.is_ruined())
                    .unwrap_or(false)
            })
            // And dung from a fold, when there is wood enough not to need it
            // on the fire (#354)
            || (agent.how_many_i_have("dung") > 0
                && agent.how_many_i_have("wood") >= Self::FIRE_BUILD_WOOD + Self::FIRE_FUEL_WOOD)
            // And compost dug out of the midden for it (#386)
            || agent.how_many_i_have(Self::COMPOST) > 0;

        if !carrying_refuse {
            return None;
        }

        // Somebody who manures fields (#419) carries it to the most worn
        // field about home that will take it, and spreads it there without
        // having to be in the mood to: they know what it does.
        if agent.knows_the_way("manure_fields") && agent.immediate_needs_met() {
            if let Some(field) = self.the_most_worn_field_about_home(agent) {
                if (field.x, field.y) == (agent_position.0, agent_position.1) {
                    return Some(Action::SpreadMuck);
                }
                return Some(Action::Move { target: (field.x, field.y, agent_position.2) });
            }
        }

        // On a field, which is where it might do some good
        let here = Position::new(agent_position.0, agent_position.1);
        let on_a_field = self
            .world
            .grid
            .get_tile(&here)
            .map(|tile| tile.terrain.is_cultivated())
            .unwrap_or(false);

        if !on_a_field {
            return None;
        }

        let curiosity = agent
            .drives
            .get(DriveType::Curiosity)
            .map(|drive| drive.value)
            .unwrap_or(0.0);

        let roll = crate::core::dice::roll().gen::<f32>();

        if agent
            .practices
            .would_try(Practice::SpreadingMuck, curiosity, roll)
        {
            return Some(Action::SpreadMuck);
        }

        None
    }

    /// The field about this one's home worn furthest below what its ground
    /// was, that muck would still bring up a rung (#419): not at the top,
    /// and none rotting in it already. The nearest home of the most worn.
    pub(in crate::analytics) fn the_most_worn_field_about_home(
        &self,
        agent: &crate::agents::Agent,
    ) -> Option<crate::world::Position> {
        use crate::world::soil::SoilGrade;
        let home = Self::where_this_one_lives(agent);
        let home_at = crate::world::Position::new(home.0, home.1);
        let rung = |grade: SoilGrade| SoilGrade::LADDER.iter().position(|g| *g == grade).unwrap_or(0) as i32;
        self.world
            .grid
            .every_field()
            .filter(|(at, _)| Self::within((at.x, at.y), home, Self::FIELD_WALK_RADIUS as i32))
            .filter(|(_, field)| field.grade < field.natural && field.mucked_ready_at.is_none())
            .min_by_key(|(at, field)| (rung(field.grade) - rung(field.natural), home_at.distance_to(at), at.x, at.y))
            .map(|(at, _)| at)
    }

    /// Scattering gathered seed on good ground near camp: Stage 1 of the
    /// agricultural path.
    ///
    /// Not a field. Nothing is broken; a handful of what was gathered is
    /// thrown down where it might come up, which is what a gathering people
    /// does once it has seen its own leavings come up as food. Done by
    /// somebody with the day's needs met and food on their mind, carrying
    /// something sown, standing on empty ground near camp, and not where the
    /// camp has already got a good many stands of its own coming up.
    pub(in crate::analytics) fn sowing_action(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> Option<Action> {
        use crate::agents::practices::Practice;
        use crate::world::Position;
        use rand::Rng;

        if !agent.knows_how_to_do("ScatterSeed") || !agent.immediate_needs_met() {
            return None;
        }

        let wants_to_provide = agent
            .drives
            .get(DriveType::Sustenance)
            .map(|drive| drive.is_active())
            .unwrap_or(false);
        if !wants_to_provide {
            return None;
        }

        Self::seed_worth_scattering(agent)?;

        let here = Position::new(agent_position.0, agent_position.1);
        let camp = self.where_the_camp_is(agent_position)?;
        if camp.distance_to(&here) > Self::A_SHORT_WALK * 2 {
            return None;
        }

        let takes_seed = self
            .world
            .grid
            .get_tile(&here)
            .map(|tile| tile.terrain.can_be_tilled() || tile.terrain.is_cultivated())
            .unwrap_or(false);
        if !takes_seed || self.world.nodes_on(here).next().is_some() {
            return None;
        }

        let sown_about_here = self
            .world
            .resources
            .iter()
            .filter(|node| node.sown_by.is_some() && node.position.distance_to(&camp) <= Self::A_SHORT_WALK * 2)
            .count();
        if sown_about_here >= Self::STANDS_A_CAMP_SOWS {
            return None;
        }

        let curiosity = agent
            .drives
            .get(DriveType::Curiosity)
            .map(|drive| drive.value)
            .unwrap_or(0.0);
        let roll = crate::core::dice::roll().gen::<f32>();
        if !agent.practices.would_try(Practice::Sowing, curiosity, roll) {
            return None;
        }

        Some(Action::ScatterSeed)
    }

    /// How many sown stands a camp keeps coming up about it before nobody
    /// bothers throwing more: a handful of patches, not a field.
    pub(in crate::analytics) const STANDS_A_CAMP_SOWS: usize = 12;

    /// Whether this one has something to sow besides what it has been
    /// eating: kept seed, grain, pods or roots (#414).
    pub(in crate::analytics) fn carrying_seed_to_sow(agent: &crate::agents::Agent) -> bool {
        Self::what_can_be_sown()
            .iter()
            .filter(|(_, crop, feeds_anybody)| *feeds_anybody && *crop != crate::world::ResourceType::Food)
            .any(|(called, _, _)| agent.how_many_i_have(called) > 0)
    }

    /// Whether this one breaks new ground, as far as keeping the fields one
    /// has goes (#417, #423).
    ///
    /// A keeper holds off while their own crops lately, or their camp's,
    /// have been lost more often than paid. Where anybody grown about their
    /// home keeps their fields, that is the camp's way: while the camp's
    /// crops are not paying, somebody who does not keep fields goes along
    /// with it and breaks none either, unless they are a rebel or stubborn,
    /// who go their own way.
    ///
    /// Not half the camp, as it was first: the new ground of a farming
    /// world goes in the second spring, a season after keeping fields first
    /// comes to anybody, when two grown people of a dozen keep them (#423).
    ///
    /// Where the ground goes is left as it was. Keeping a camp's fields
    /// about home as well, seed or no seed, cost a seventh of the field crop
    /// brought in over forty worlds and a third of it in the worst: a field
    /// sown with seed a day out is picked by whoever passes, and half of
    /// what comes in comes off such fields (#414, #423).
    pub(in crate::analytics) fn how_this_one_keeps_the_fields(&self, agent: &crate::agents::Agent) -> KeepingFields {
        use crate::core::traits::Trait;

        let camp = self.who_lives_near(Self::where_this_one_lives(agent));
        let keepers = camp
            .iter()
            .filter(|&&index| self.population.agents[index].knows_the_way("keep_fields"))
            .count();
        let the_camps_crops: Vec<bool> = camp
            .iter()
            .flat_map(|&index| self.population.agents[index].crops_lately.iter().copied())
            .collect();
        let the_camps_crops_pay = crate::agents::Agent::do_these_crops_pay(&the_camps_crops);

        if agent.knows_the_way("keep_fields") {
            if !agent.are_my_crops_paying() || !the_camps_crops_pay {
                return KeepingFields::HoldsOff;
            }
            return KeepingFields::Breaks;
        }

        let the_camps_way = keepers > 0;
        if !the_camps_way {
            return KeepingFields::Breaks;
        }
        if agent.traits.has(Trait::Rebel) || agent.traits.has(Trait::Stubborn) {
            return KeepingFields::Breaks;
        }
        if !the_camps_crops_pay {
            return KeepingFields::GoesAlong;
        }
        KeepingFields::Breaks
    }

    /// Breaking ground, and walking to somewhere worth breaking.
    ///
    /// Wild food regrows about four times slower than a grown settlement eats
    /// it, which is why settlements that got past a dozen people starved back
    /// down again. A field yields many times what the same ground does wild,
    /// and this is how one comes to exist: an agent with the immediate needs
    /// answered and the Sustenance drive up on it goes and breaks ground.
    pub(in crate::analytics) fn farming_action(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> Option<Action> {
        // Only somebody with nothing more pressing on. The drive itself only
        // climbs in an agent that is fed, watered, rested and warm.
        if !agent.immediate_needs_met() {
            return None;
        }

        // A garden is Stage 2 of the agricultural path: found out by somebody
        // who has scattered seed and gathered from what came up, or shown by
        // somebody who has (`Agent::ACTIONS_FOUND_OUT_BY_THEMSELVES`).
        if !agent.knows_how_to_do("TillSoil") {
            return None;
        }

        let wants_to_provide = agent
            .drives
            .get(DriveType::Sustenance)
            .map(|drive| drive.is_active())
            .unwrap_or(false);

        if !wants_to_provide {
            return None;
        }

        // A standing field that has gone over to weeds is worth more than a
        // new one. "Farmers should not just drop seeds and get crops" - a
        // field neglected for a season carries almost nothing, and going round
        // it pulling weeds and picking pests off is most of what growing a
        // crop consists of.
        if let Some(field) = self.field_wanting_work(agent_position) {
            if field.x == agent_position.0 && field.y == agent_position.1 {
                return Some(Action::TendField);
            }

            return Some(Action::Move {
                target: (field.x, field.y, agent_position.2),
            });
        }

        // Enough fields around here already
        if self.fields_within(agent_position, Self::FIELD_WALK_RADIUS) >= Self::FIELDS_WANTED {
            return None;
        }

        // And breaking new ground is a thing that has to be worked out first.
        // Until an agent has seen food come up out of ground somebody put seed
        // in, spending a day digging grass over is a strange way to answer
        // hunger, and it does it only out of curiosity.
        let curiosity = agent
            .drives
            .get(DriveType::Curiosity)
            .map(|drive| drive.value)
            .unwrap_or(0.0);

        let roll = {
            use rand::Rng;
            crate::core::dice::roll().gen::<f32>()
        };

        if !agent.practices.would_try(
            crate::agents::practices::Practice::Farming,
            curiosity,
            roll,
        ) {
            return None;
        }

        // A stand of pods on tired ground is worth turning under before it is
        // worth breaking fresh ground somewhere else - see
        // `Simulation::ploughing_a_crop_in`. Asked first, because a man who
        // would break ground at all is a man who would plough a crop in, and
        // because the ground he already has is nearer than the ground he has
        // not.
        if let Some(under) = self.a_stand_worth_turning_under(agent, agent_position) {
            if under.x == agent_position.0 && under.y == agent_position.1 {
                return Some(Action::TillSoil);
            }

            return Some(Action::Move {
                target: (under.x, under.y, agent_position.2),
            });
        }

        // Somebody who keeps a planting calendar breaks no ground for a crop
        // out of its time (#418): with nothing to sow that would have most of
        // its season ahead of it, the ground waits.
        if Self::what_this_one_would_sow_on(agent, 1.0, Some(self.world.climate.calendar.day_of_year)).is_none() {
            self.sowings_put_off.set(self.sowings_put_off.get() + 1);
            return None;
        }

        // A farmer who keeps the fields they have (#417) breaks no more
        // while their own crops lately have been lost more often than they
        // paid: a field is time and seed put in against what comes off it,
        // and another one when the ones there are going to waste is more of
        // both thrown after them. They tend and bring in what they have.
        //
        // And not while their camp's are (#423). A keeper whose own crops
        // paid used to go on breaking ground beside a camp whose crops were
        // falling, and a third of the camp who did not keep fields broke
        // most of the new ground there was: a keeper held back forty-six
        // thousand times a draw and the camp broke as many fields as
        // before. Where anybody grown in the camp keeps their fields, it is
        // the camp's way, and the rest go along with it unless they are the
        // kind who go their own way.
        match self.how_this_one_keeps_the_fields(agent) {
            KeepingFields::HoldsOff => {
                self.ground_left_unbroken.set(self.ground_left_unbroken.get() + 1);
                return None;
            }
            KeepingFields::GoesAlong => {
                self.ground_left_by_the_camps_way.set(self.ground_left_by_the_camps_way.get() + 1);
                return None;
            }
            KeepingFields::Breaks => {}
        }

        // Seed before ground. A farmer sows what he is carrying, and grain
        // comes first of anything he might carry - and he never carried it,
        // because foraging takes the nearest food and grain stands in a few
        // patches against many of berries: 3 fields of grain in 691 (#10). So
        // somebody about to break ground with no grain on him, who knows of
        // some standing within a walk, fetches a handful first.
        if let Some(seed) = self.wild_grain_to_sow_from(agent, agent_position) {
            if seed.x == agent_position.0 && seed.y == agent_position.1 {
                return Some(Action::Gather { resource_type: "grain".to_string() });
            }

            return Some(Action::Move {
                target: (seed.x, seed.y, agent_position.2),
            });
        }

        // Ground broken with seed in hand is worth breaking where one
        // stands: grain, pods and roots sown a day's walk out are picked by
        // whoever passes, and half of all field crop brought in came off
        // fields more than a walk from the gatherer's hearth (#414). Ground
        // broken with nothing to sow but what one has been eating is not. It
        // comes up the same as the berries and greens all round it, and
        // nobody brought in a mouthful off one more than a walk from home,
        // where only the harvest goes looking for it - three fields in four
        // of these were sown out there, and ripened and fell. So somebody
        // with nothing to sow breaks ground only about home, and only within
        // a walk of it.
        //
        // But not past two walks from home (#425): that ground is sown and
        // left, and fourteen and a half units fell there for every one
        // brought in (#424).
        let ground = if Self::carrying_seed_to_sow(agent) {
            let reach = Self::FIELDS_ARE_BROKEN_WITHIN + Self::FIELD_WALK_RADIUS as i32;
            if !Self::within((agent_position.0, agent_position.1), Self::where_this_one_lives(agent), reach) {
                self.ground_left_too_far_out.set(self.ground_left_too_far_out.get() + 1);
                return None;
            }
            let ground = self.ground_to_break(agent_position)?;
            if Self::too_far_out_to_break(agent, (ground.x, ground.y)) {
                self.ground_left_too_far_out.set(self.ground_left_too_far_out.get() + 1);
                return None;
            }
            ground
        } else {
            let home = Self::where_this_one_lives(agent);
            if !Self::within((agent_position.0, agent_position.1), home, Self::FIELD_WALK_RADIUS as i32) {
                return None;
            }
            let at_home = (home.0, home.1, agent_position.2);
            if self.fields_within(at_home, Self::FIELD_WALK_RADIUS) >= Self::FIELDS_WANTED {
                return None;
            }
            self.ground_to_break(at_home)?
        };

        if ground.x == agent_position.0 && ground.y == agent_position.1 {
            return Some(Action::TillSoil);
        }

        Some(Action::Move {
            target: (ground.x, ground.y, agent_position.2),
        })
    }

    /// The nearest wild grain this one knows of with something on it, for
    /// somebody about to sow with none in the pack.
    ///
    /// Only within `A_WALK_FOR_SEED`: a handful of seed is worth an errand,
    /// not an expedition.
    pub(in crate::analytics) fn wild_grain_to_sow_from(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> Option<crate::world::Position> {
        use crate::world::{Position, ResourceType};

        if agent.how_many_i_have("grain") > 0
            || agent.how_many_i_have("sproutedgrain") > 0
            || agent.how_many_i_have(crate::agents::Agent::SEED) > 0
        {
            return None;
        }

        let here = Position::new(agent_position.0, agent_position.1);
        agent
            .exploration_knowledge
            .known_resources
            .iter()
            .filter(|(_, kind)| **kind == ResourceType::Grain)
            .map(|(at, _)| *at)
            .filter(|at| here.distance_to(at) <= Self::A_WALK_FOR_SEED)
            .filter(|at| {
                self.world
                    .nodes_on(*at)
                    .any(|node| node.resource_type == ResourceType::Grain && node.amount > 0)
            })
            .min_by_key(|at| here.distance_to(at))
    }

    /// How far somebody will go for seed before breaking ground: twice the
    /// walk to a field.
    pub(in crate::analytics) const A_WALK_FOR_SEED: u32 = 2 * Self::FIELD_WALK_RADIUS;

    /// The nearest stand worth turning under: a ground-feeding crop on a field
    /// its farmer thinks poor, or anything at all on a field he thinks worn to
    /// nothing.
    ///
    /// Both halves of the first matter. A pod row on good ground is food and
    /// should be picked; a pod row on ground that will not carry a crop is
    /// worth more under the plough than in a basket, and that is the whole
    /// judgement a green manure asks for. The second is how a field changes
    /// crop: what stands on ground at the bottom of the ladder is not worth
    /// picking, and the ground wants something else in it.
    ///
    /// What he thinks, not what the ground is: the grade of a field is
    /// something a farmer works out from what has come off it - see
    /// `Agent::saw_a_stand_on` - and a field he has never seen a crop on he
    /// has no opinion of, and leaves alone.
    pub(in crate::analytics) fn a_stand_worth_turning_under(
        &self,
        agent: &crate::agents::Agent,
        position: (i32, i32, i32),
    ) -> Option<crate::world::Position> {
        use crate::world::resources::ResourceNode;
        use crate::world::{Position, SoilGrade};

        let from = Position::new(position.0, position.1);
        let mut best: Option<(Position, u32)> = None;

        for resource in self.world.nodes_near(from, Self::FIELD_WALK_RADIUS) {
            let distance = from.distance_to(&resource.position);
            if distance > Self::FIELD_WALK_RADIUS {
                continue;
            }

            let Some(thought) =
                agent.what_i_make_of_the_field_at((resource.position.x, resource.position.y))
            else {
                continue;
            };

            let poor = ResourceNode::WHAT_ORDINARY_WILD_GROUND_CARRIES * thought.multiplier()
                < Self::TOO_POOR_FOR_A_HUNGRY_CROP;
            let worn_to_nothing = thought == SoilGrade::LADDER[0];

            let worth_it = (resource.resource_type.feeds_the_ground() && resource.amount > 0 && poor)
                || worn_to_nothing;

            if !worth_it {
                continue;
            }

            if best.map(|(_, so_far)| distance < so_far).unwrap_or(true) {
                best = Some((resource.position, distance));
            }
        }

        best.map(|(where_it_stands, _)| where_it_stands)
    }

    /// How near the camp a plant has to stand before nobody would bother
    /// moving it, and how near a cutting has to be put in for the move to have
    /// been worth making.
    pub(in crate::analytics) const A_SHORT_WALK: u32 = 6;

    /// How many people standing together make a camp, when there is no roof up
    /// yet to mark one.
    pub(in crate::analytics) const ENOUGH_PEOPLE_TO_BE_A_CAMP: u32 = 3;

    /// How much of a plant comes away as a cutting, and how much the slip
    /// grows into once it is in.
    ///
    /// The first cut of this took three units off the parent and grew into a
    /// plant carrying forty, and left the parent as big as it was. Over eight
    /// worlds the people planted two hundred slips apiece and the food
    /// standing on the map went up six times: transplanting was not moving
    /// food about, it was manufacturing it out of nothing.
    ///
    /// A slip is a piece of the plant. It comes off the parent's carrying
    /// capacity and not only off this year's crop, and what it grows into is
    /// somewhat more than what it cost - because a plant put in open ground
    /// with nobody's roots against it does better than one more stem on a
    /// crowded patch. Somewhat, not thirteen times.
    pub(in crate::analytics) const WHAT_A_CUTTING_TAKES: u32 = 8;
    pub(in crate::analytics) const WHAT_A_CUTTING_STARTS_WITH: u32 = 2;
    pub(in crate::analytics) const WHAT_A_MOVED_PLANT_COMES_TO: u32 = 20;

    /// And how small a patch has to be before nobody digs any more out of it.
    pub(in crate::analytics) const TOO_THIN_TO_DIG: u32 = 12;

    /// Moving a plant that is known to be good to ground beside the camp.
    ///
    /// This is the third way into farming and the one that needs no seed and
    /// no theory at all. A person who walks half a morning to the same berry
    /// bush every day, and who has already dug up plants for one reason or
    /// another, eventually digs up that one and puts it in beside the tents.
    /// It is not an idea about agriculture. It is an idea about the walk.
    ///
    /// Two halves: lift a piece of something growing a long way off, and put
    /// it in the ground where you live. What it teaches is taught by the
    /// plant standing there afterwards, which is `record_outcome` on the
    /// harvest like any other crop.
    pub(in crate::analytics) fn transplanting_action(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> Option<Action> {
        use crate::world::Position;

        // Propagation from cuttings is Stage 2 of the agricultural path, and
        // comes with gardens
        if !agent.knows_how_to_do("PlantCutting") {
            return None;
        }

        let here = Position::new(agent_position.0, agent_position.1);
        let camp = self.where_the_camp_is(agent_position)?;

        // Carrying one already: get it in the ground somewhere near home
        if let Some(cutting) = Self::a_cutting_in_the_pack(agent) {
            let _ = cutting;

            if camp.distance_to(&here) > Self::A_SHORT_WALK {
                return Some(Action::Move {
                    target: (camp.x, camp.y, agent_position.2),
                });
            }

            let can_carry_it = self
                .world
                .grid
                .get_tile(&here)
                .map(|tile| tile.terrain.can_be_tilled() || tile.terrain.is_cultivated())
                .unwrap_or(false);

            let taken = self.world.nodes_on(here).next().is_some();

            if can_carry_it && !taken {
                return Some(Action::PlantCutting);
            }

            // Standing on the wrong tile at home: step to one that will do
            let spot = self.ground_to_break((camp.x, camp.y, agent_position.2))?;
            if spot != here {
                return Some(Action::Move {
                    target: (spot.x, spot.y, agent_position.2),
                });
            }

            return None;
        }

        // Nothing carried: lift a piece of whatever is standing here, if it is
        // worth lifting - which means it is a long way from home and there is
        // something growing here that is known to be good
        if camp.distance_to(&here) <= Self::A_SHORT_WALK {
            return None;
        }

        // The first thing growing here that is worth lifting - not the first
        // thing growing here. A tile can carry more than one, and a strange
        // plant nobody has tried standing on the same ground as a berry bush
        // was enough to hide the bush.
        self.world
            .nodes_on(here)
            .filter(|resource| {
                resource.amount > Self::WHAT_A_CUTTING_TAKES
                    && resource.max_amount > Self::TOO_THIN_TO_DIG + Self::WHAT_A_CUTTING_TAKES
            })
            .find(|resource| {
                Self::what_can_be_sown()
                    .into_iter()
                    .any(|(_, crop, _)| crop == resource.resource_type)
            })
            .map(|_| Action::TakeCutting)
    }

    /// How often a curious man with a pack full of parts tries putting the
    /// wrong one in the right place.
    ///
    /// Low, and it is meant to be. Each try costs the makings of a spear, and
    /// the great majority of them come to nothing at all.
    pub(in crate::analytics) const HOW_OFTEN_ANYBODY_TRIES_A_SWAP: f64 = 0.04;

    /// How willing somebody has to be before they put a strange plant in
    /// their mouth.
    ///
    /// Set against the Curiosity drive rather than a trait, because this is a
    /// thing done on an idle afternoon by somebody with nothing pressing on
    /// them - never by a man with a wolf behind him, and only rarely by
    /// anybody.
    pub(in crate::analytics) const CURIOUS_ENOUGH_TO_EAT_IT: f32 = 0.55;

    /// And how often even a curious man actually does it, per chance.
    ///
    /// Low. A person who walks past a strange plant every day for years
    /// eventually tries one; a person who tries every plant he passes does not
    /// get to be a person for long.
    ///
    /// This is the chance of setting out towards one, not of eating it: a man
    /// who has walked to the plant eats it. Rolling again on arrival, which is
    /// what the first cut did, compounded a small chance against itself once
    /// per turn of the walk and meant nobody in eight worlds ever arrived.
    pub(in crate::analytics) const HOW_OFTEN_ANYBODY_RISKS_IT: f64 = 0.06;

    /// How well somebody has to be to try a strange plant: well enough that
    /// the worst one there is leaves them standing.
    pub(in crate::analytics) const WELL_ENOUGH_TO_RISK_IT: f32 =
        Self::WHAT_A_BAD_PLANT_DOES.1 + 15.0;

    /// Trying an unknown plant.
    pub(in crate::analytics) fn tasting_action(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> Option<Action> {
        use crate::world::{Position, ResourceType};
        use rand::Rng;

        // Not while anything is actually wrong. A hungry man eating a strange
        // plant is a different story and a worse one; this is the idle
        // curiosity that finds things out cheaply.
        if !agent.immediate_needs_met() {
            return None;
        }

        // Nor by somebody who is not well. The worst a plant does is what
        // `WHAT_A_BAD_PLANT_DOES` says, and it was written so that the top of
        // it "kills somebody who was not in good condition to start with" -
        // but nothing asked what condition the taster was in, and a winter
        // leaves everybody in poor condition. Over twelve settlement-years,
        // **eighteen people died of a mouthful of something strange**, one
        // and a half a settlement a year: an eighth of a people of twelve,
        // every year, to idle curiosity. Somebody well enough that the worst
        // plant there is leaves them standing tries things; somebody who is
        // not, waits. See ISSUES_FOUND #252.
        if agent.state.health < Self::WELL_ENOUGH_TO_RISK_IT {
            return None;
        }

        let curious = agent
            .drives
            .get(DriveType::Curiosity)
            .map(|drive| drive.value)
            .unwrap_or(0.0);

        if curious < Self::CURIOUS_ENOUGH_TO_EAT_IT {
            return None;
        }

        let here = Position::new(agent_position.0, agent_position.1);

        // The nearest one of a sort nobody here has an opinion about. The
        // first cut of this asked the agent to be standing exactly on the
        // plant, and over eight worlds of ten thousand turns not one person
        // ever tried anything: sixteen tiles in ten thousand is not a thing
        // that happens by accident.
        let strange = self
            .nodes_known_to(agent, here, Self::FORAGE_RADIUS)
            .filter(|resource| {
                resource.resource_type == ResourceType::StrangePlant && resource.amount > 0
            })
            .filter(|resource| !agent.have_i_tried_that_plant(resource.kind))
            .map(|resource| (resource.position, here.distance_to(&resource.position)))
            .filter(|(_, apart)| *apart <= Self::FORAGE_RADIUS)
            .min_by_key(|(_, apart)| *apart)
            .map(|(where_it_is, _)| where_it_is)?;

        // Standing on it already: the walk was the deciding, and re-deciding
        // on arrival is what kept anybody from ever getting there. The roll is
        // made once, to set out.
        if strange == here {
            return Some(Action::Taste);
        }

        if !crate::core::dice::roll().gen_bool(Self::HOW_OFTEN_ANYBODY_RISKS_IT) {
            return None;
        }

        Some(Action::Move {
            target: (strange.x, strange.y, agent_position.2),
        })
    }

    /// What a cutting of a named crop is called in a pack
    pub(in crate::analytics) fn a_cutting_of(called: &str) -> String {
        format!("{called}cutting")
    }

    /// The cutting this agent is carrying, if any
    pub(in crate::analytics) fn a_cutting_in_the_pack(
        agent: &crate::agents::Agent,
    ) -> Option<(&'static str, crate::world::ResourceType)> {
        Self::what_can_be_sown()
            .into_iter()
            .find(|(called, _, _)| agent.how_many_i_have(&Self::a_cutting_of(called)) > 0)
            .map(|(called, crop, _)| (called, crop))
    }

    /// How near the end of its season a ripe crop is before somebody who
    /// keeps the seasons in mind goes out to bring it in, hungry or not
    /// (#395): a month.
    pub(in crate::analytics) const HARVEST_BEFORE_THE_SEASON_IS_OUT: u32 = 30;

    /// How much has to be standing to be taken before a harvest is worth
    /// the walk, at an ordinary bother about waste (#395): an armful or two.
    /// The frugal go for less; the lazy wait for more.
    pub(in crate::analytics) const A_HARVEST_WORTH_THE_WALK: f64 = 6.0;

    /// The ripe crop of the camp's this one would bring in now, for somebody
    /// who keeps the seasons in mind (#395): ripe, within a walk of home,
    /// its season out within a month, and enough of it to be worth the walk
    /// to them. The nearest.
    pub(in crate::analytics) fn a_harvest_to_bring_in(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> Option<(crate::world::Position, &'static str)> {
        use crate::world::Position;

        if !agent.knows_the_way("seasons") || !agent.immediate_needs_met() {
            return None;
        }
        let home = Self::where_this_one_lives(agent);
        let today = self.world.climate.calendar.day_of_year;
        let here = Position::new(agent_position.0, agent_position.1);
        let bother = agent.how_much_waste_bothers_me();
        self.world
            .nodes_near(Position::new(home.0, home.1), Self::FIELD_WALK_RADIUS)
            .filter(|crop| crop.on_a_field && crop.ripe_stand > 0)
            .filter(|crop| {
                crop.resource_type
                    .bearing_window()
                    .days_left(today)
                    .is_some_and(|left| left <= Self::HARVEST_BEFORE_THE_SEASON_IS_OUT)
            })
            .filter(|crop| crop.what_can_be_taken() as f64 * bother >= Self::A_HARVEST_WORTH_THE_WALK)
            .filter_map(|crop| {
                Self::edible_item_for(crop.resource_type)?;
                Some((crop.position, Self::gathered_as(crop.resource_type)?))
            })
            .min_by_key(|(at, _)| here.distance_to(at))
    }

    /// The next of bringing a harvest in (#395): to the crop, or take it.
    pub(in crate::analytics) fn harvest_action(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> Option<Action> {
        let (at, called) = self.a_harvest_to_bring_in(agent, agent_position)?;
        if (at.x, at.y) == (agent_position.0, agent_position.1) {
            return Some(Action::Gather { resource_type: called.to_string() });
        }
        Some(Action::Move { target: (at.x, at.y, agent_position.2) })
    }

    /// How many fields an edge would take in before somebody would think it
    /// worth the brush, at an ordinary bother about waste (#398): two. The
    /// frugal edge one; the lazy, seven.
    pub(in crate::analytics) const FIELDS_WORTH_AN_EDGE: f64 = 2.0;

    /// Whether a closed hedge, or an edge begun round fields, takes this
    /// place in already (#398).
    pub(in crate::analytics) fn is_this_field_hedged(&self, at: (i32, i32)) -> bool {
        let now = self.world.turn;
        self.world
            .thorn_hedges
            .iter()
            .any(|hedge| (hedge.is_closed(now) || (hedge.round_fields && hedge.closed_on.is_none())) && hedge.takes_in(at))
    }

    /// The field of the camp's an edge would go round next, and how many
    /// open fields it would take in (#398): an edge begun and not closed, or
    /// else the open field nearest home, if the open fields about it are
    /// worth the brush to this one.
    pub(in crate::analytics) fn a_field_to_hedge(&self, agent: &crate::agents::Agent) -> Option<crate::world::Position> {
        use crate::world::{Position, ThornHedge};

        let home = Self::where_this_one_lives(agent);
        let near_home = |at: (i32, i32)| Self::within(at, home, Self::FIELD_WALK_RADIUS as i32);

        if let Some(begun) = self
            .world
            .thorn_hedges
            .iter()
            .find(|hedge| hedge.round_fields && hedge.closed_on.is_none() && near_home((hedge.around.x, hedge.around.y)))
        {
            return Some(begun.around);
        }

        let open_field = |at: (i32, i32)| {
            self.world.grid.get_tile(&Position::new(at.0, at.1)).is_some_and(|tile| tile.terrain.is_cultivated())
                && !self.is_this_field_hedged(at)
        };
        let reach = Self::FIELD_WALK_RADIUS as i32;
        let mut nearest: Option<((i32, i32), i32)> = None;
        for dx in -reach..=reach {
            for dy in -reach..=reach {
                let at = (home.0 + dx, home.1 + dy);
                if !open_field(at) {
                    continue;
                }
                let apart = dx.abs().max(dy.abs());
                if nearest.map_or(true, |(_, so_far)| apart < so_far) {
                    nearest = Some((at, apart));
                }
            }
        }
        let (at, _) = nearest?;

        let edge = ThornHedge::HOW_FAR_A_FIELD_EDGE_GOES;
        let mut it_takes_in = 0;
        for dx in -edge..=edge {
            for dy in -edge..=edge {
                if open_field((at.0 + dx, at.1 + dy)) {
                    it_takes_in += 1;
                }
            }
        }
        (it_takes_in as f64 * agent.how_much_waste_bothers_me() >= Self::FIELDS_WORTH_AN_EDGE)
            .then_some(Position::new(at.0, at.1))
    }

    /// The next of hedging the camp's fields, for somebody who knows to
    /// (#398): to the field an edge goes round with brush in hand and lay
    /// it, or fetch brush for it.
    pub(in crate::analytics) fn hedging_action(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> Option<Action> {
        if !agent.knows_the_way("hedge_fields")
            || !agent.immediate_needs_met()
            || agent.state.years_old() < crate::agents::Agent::OLD_ENOUGH_TO_KEEP_A_KIT
        {
            return None;
        }
        let field = self.a_field_to_hedge(agent)?;
        if agent.how_many_i_have("wood") == 0 {
            return self
                .nearest_resource_within(agent, agent_position, Self::FORAGE_RADIUS, |resource| {
                    resource.resource_type == crate::world::ResourceType::Wood
                })
                .map(|_| Action::Gather { resource_type: "wood".to_string() });
        }
        if (agent_position.0, agent_position.1) == (field.x, field.y) {
            return Some(Action::HedgeAField);
        }
        Some(Action::Move { target: (field.x, field.y, agent_position.2) })
    }

    /// The nearest field within reach that has gone over to weeds and pests
    pub(in crate::analytics) fn field_wanting_work(
        &self,
        position: (i32, i32, i32),
    ) -> Option<crate::world::Position> {
        use crate::world::Position;

        let from = Position::new(position.0, position.1);
        let reach = Self::FIELD_WALK_RADIUS as i32;

        let mut best: Option<(Position, u32)> = None;

        for dx in -reach..=reach {
            for dy in -reach..=reach {
                let candidate = Position::new(from.x + dx, from.y + dy);

                let Some(tile) = self.world.grid.get_tile(&candidate) else {
                    continue;
                };

                if !tile.terrain.is_cultivated() || !tile.soil.wants_working() {
                    continue;
                }

                let distance = from.distance_to(&candidate);
                if distance > Self::FIELD_WALK_RADIUS {
                    continue;
                }

                if best.map(|(_, apart)| distance < apart).unwrap_or(true) {
                    best = Some((candidate, distance));
                }
            }
        }

        best.map(|(where_it_is, _)| where_it_is)
    }
}

/// What keeping the fields one has comes to for somebody about to break
/// ground (#417, #423).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::analytics) enum KeepingFields {
    /// They keep their fields, and theirs or their camp's crops are not
    /// paying: no new ground.
    HoldsOff,
    /// They do not keep fields, but their camp does and its crops are not
    /// paying, and they go along with it: no new ground.
    GoesAlong,
    /// The crops pay, or nobody about keeps fields, or they go their own
    /// way: new ground as before.
    Breaks,
}
