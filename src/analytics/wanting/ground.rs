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
    pub(in crate::analytics) fn what_can_be_sown() -> [(&'static str, crate::world::ResourceType, bool); 6] {
        use crate::world::ResourceType;

        // The flag is whether it is worth breaking ground for when the thing
        // driving you is hunger. Sprouted grain comes first because it is the
        // one thing in the list that is visibly already doing what a field is
        // for - a man holding it does not have to be told.
        [
            ("sproutedgrain", ResourceType::Grain, true),
            ("grain", ResourceType::Grain, true),
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
    pub(in crate::analytics) fn what_this_one_would_sow(agent: &crate::agents::Agent) -> crate::world::ResourceType {
        use crate::world::ResourceType;

        let mut best: Option<(ResourceType, f32)> = None;

        for (called, crop, feeds_anybody) in Self::what_can_be_sown() {
            // A field is broken to answer hunger, so what goes in it is
            // something a person can eat. The first cut of this let an agent
            // sow whatever was in the pack, and over eight worlds the people
            // put in flax and cotton and starved beside their own linen.
            if !feeds_anybody {
                continue;
            }

            if agent.how_many_i_have(called) == 0 {
                continue;
            }

            let believed = agent
                .lessons
                .how_likely_to_try_this(&format!("sow:{called}"));

            if best.map(|(_, so_far)| believed > so_far).unwrap_or(true) {
                best = Some((crop, believed));
            }
        }

        best.map(|(crop, _)| crop).unwrap_or(ResourceType::Food)
    }

    /// How much comes up off one tile's worth of seed.
    pub(in crate::analytics) const WHAT_A_MIDDEN_COMES_UP_IN: f32 = 8.0;

    /// How wet it has to be under a pack before grain in it starts moving.
    ///
    /// Set against `Soil::humidity`, which reads the country and the sky
    /// together: a wetland or a riverbank is wet enough standing still, a
    /// forest floor is on the line, and open plains only get there when it is
    /// actually raining. Dry ground under a clear sky never does.
    pub(in crate::analytics) const WET_ENOUGH_TO_START_IT: f32 = 0.7;

    /// And how readily it goes, per tick, at that wetness.
    ///
    /// Slow: a handful of grain that gets rained on does not come up the same
    /// afternoon. Over a wet season most of what a person is carrying will
    /// have started, which is the point - the seed spoils as food and becomes
    /// something else.
    pub(in crate::analytics) const HOW_READILY_GRAIN_TAKES: f32 = 0.01;

    /// Fields already broken within reach
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

                if self
                    .world
                    .grid
                    .get_tile(&candidate)
                    .map(|tile| tile.terrain.is_cultivated())
                    .unwrap_or(false)
                {
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
        // agent per tick
        let occupied: std::collections::BTreeSet<(i32, i32)> = self
            .world
            .resources
            .iter()
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

                let tillable = self
                    .world
                    .grid
                    .get_tile(&candidate)
                    .map(|tile| tile.terrain.can_be_tilled())
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
            });

        if !carrying_refuse {
            return None;
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

        let ground = self.ground_to_break(agent_position)?;

        if ground.x == agent_position.0 && ground.y == agent_position.1 {
            return Some(Action::TillSoil);
        }

        Some(Action::Move {
            target: (ground.x, ground.y, agent_position.2),
        })
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

            let taken = self
                .world
                .resources
                .iter()
                .any(|resource| resource.position == here);

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
            .resources
            .iter()
            .filter(|resource| {
                resource.position == here
                    && resource.amount > Self::WHAT_A_CUTTING_TAKES
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
    /// per tick of the walk and meant nobody in eight worlds ever arrived.
    pub(in crate::analytics) const HOW_OFTEN_ANYBODY_RISKS_IT: f64 = 0.06;

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
        // plant, and over eight worlds of ten thousand ticks not one person
        // ever tried anything: sixteen tiles in ten thousand is not a thing
        // that happens by accident.
        let strange = self
            .world
            .resources
            .iter()
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
