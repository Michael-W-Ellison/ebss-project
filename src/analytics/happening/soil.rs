// src/analytics/happening/soil.rs
//! What the ground does, and what goes back into it.
//!
//! A midden coming up in berries, grain sprouting in a wet pack, a cutting
//! taking root where it was thrown, the ground telling on whoever camped on
//! it, and everything the living and the dead leave behind.
//!
//! Part of what happens whether or not anybody decides anything - see
//! [`super`]. Called from [`crate::analytics::turn`], in the order argued over
//! there.

use super::super::Simulation;
use log::debug;

impl Simulation {
    /// What a midden turns into, once it has stopped being a midden.
    ///
    /// "If the agents are expelling their waste and piling it away from their
    /// tents, then over time the waste should break down and seeds from the
    /// plants they have eaten should sprout."
    ///
    /// Everything it needs is already on the tile: the seeds that came through
    /// whole, the nutrient the rot released, and enough time for the smell to
    /// go. When all three line up something comes up, and it is food, and
    /// nobody planted it.
    pub(in crate::analytics) fn what_was_dropped_comes_up(&mut self) {
        use crate::world::{Position, ResourceNode, ResourceType};

        let mut came_up: Vec<Position> = Vec::new();

        // The ground that has something on it, rather than every tile in the
        // world - see `Grid::note_something_on`. Seed only ever arrives with
        // waste, which notes the tile as it lands.
        for at in self.world.grid.where_the_ground_is_doing_something() {
            if self
                .world
                .grid
                .get_tile(&at)
                .is_some_and(|tile| tile.soil.ready_to_sprout())
            {
                came_up.push(at);
            }
        }

        for where_it_is in came_up {
            // Not on top of something already growing there.
            if self
                .world
                .resources
                .iter()
                .any(|resource| resource.position == where_it_is)
            {
                continue;
            }

            let seed = match self.world.grid.get_tile_mut(&where_it_is) {
                Some(tile) => tile.soil.it_came_up(),
                None => continue,
            };

            // What comes up is a volunteer, not a field: a few plants off one
            // midden, and no bigger for a bigger midden.
            let how_much = ((seed * Self::WHAT_A_MIDDEN_COMES_UP_IN).round() as u32).clamp(1, 8);

            let mut volunteer =
                ResourceNode::new(ResourceType::Food, where_it_is, how_much);
            volunteer.amount = how_much;
            self.world.resources.push(volunteer);

            debug!("Something came up on the midden at {where_it_is:?}");

            // And whoever is close enough to see it takes the lesson: what
            // they threw away last season is standing here as food. This is
            // the only thing in the world that teaches farming outright -
            // everything else is somebody breaking ground on a hunch.
            for agent in self
                .population
                .agents
                .iter_mut()
                .filter(|agent| agent.body.is_alive())
            {
                let apart = (agent.state.position.0 - where_it_is.x).abs()
                    + (agent.state.position.1 - where_it_is.y).abs();

                if apart <= Self::CLOSE_ENOUGH_TO_SEE_IT_COME_UP {
                    agent
                        .practices
                        .saw_it_work(crate::agents::practices::Practice::Farming);
                }
            }
        }
    }

    /// What a fair trade is worth to a bond, and what a gift is.
    ///
    /// A gift is worth more, which is the whole difference between the two:
    /// a trade leaves both parties square and a gift leaves one of them owing.
    pub(in crate::analytics) const WHAT_A_FAIR_TRADE_IS_WORTH: f32 = 0.15;
    pub(in crate::analytics) const WHAT_A_GIFT_IS_WORTH: f32 = 0.4;

    /// How near two people have to be standing to hand anything over.
    pub(in crate::analytics) const CLOSE_ENOUGH_TO_HAND_SOMETHING_OVER: i32 = 3;

    /// Grain carried in the wet stops being grain.
    ///
    /// "Something like grain getting wet should result in the grains
    /// sprouting." Nobody does this on purpose. It is a thing that happens to
    /// a pack in the rain, and it is the plainest lesson in the world about
    /// what seed does, because it happens in the owner's hands.
    pub(in crate::analytics) fn what_got_wet_sprouts(&mut self) {
        use crate::agents::InventoryItem;
        use rand::Rng;

        let raining = self
            .world
            .climate
            .weather
            .weather_type
            .precipitation_intensity();

        let mut rng = crate::core::dice::roll();

        for index in 0..self.population.agents.len() {
            if !self.population.agents[index].state.is_alive {
                continue;
            }

            let where_it_stands = self.population.agents[index].state.position;

            // Rain on the pack, or the wet of the ground it is set down on.
            // A camp beside a river is a wet camp whatever the sky is doing.
            let wet = self
                .world
                .grid
                .get_tile(&crate::world::Position::new(where_it_stands.0, where_it_stands.1))
                .map(|tile| {
                    crate::world::Soil::humidity(tile.terrain.terrain_type, raining)
                })
                .unwrap_or(0.0);

            if wet < Self::WET_ENOUGH_TO_START_IT {
                continue;
            }

            let agent = &mut self.population.agents[index];

            if agent.how_many_i_have("grain") == 0 {
                continue;
            }

            if !rng.gen_bool((wet * Self::HOW_READILY_GRAIN_TAKES).clamp(0.0, 1.0) as f64) {
                continue;
            }

            agent.inventory.remove_item("grain", 1);
            agent.inventory.add_item(InventoryItem::new_with_weight(
                "sproutedgrain".to_string(),
                1,
                0.5,
            ));

            debug!("Agent {} found the grain in its pack coming up", agent.id);
        }
    }

    /// How readily a sprouted grain works its way out of a pack, per tick.
    pub(in crate::analytics) const WHAT_FALLS_OUT_OF_A_PACK: f64 = 0.02;

    /// And what a plant grown from one carries when it is full grown.
    pub(in crate::analytics) const WHAT_ONE_SEED_COMES_TO: u32 = 30;

    /// A sprouted grain dropped where it can grow, grows.
    ///
    /// "If sprouted grains are thrown out or dropped, they could grow into
    /// adult plants." Nobody plants this. It falls out of a pack onto ground
    /// somebody happened to be standing on, and the next time anybody walks
    /// past there is a plant. Whoever is near enough to see it takes the
    /// lesson, the same as with the midden - this is the second of the two
    /// accidents that teach a people what seed is for.
    pub(in crate::analytics) fn what_was_dropped_takes_root(&mut self) {
        use crate::world::{Position, ResourceNode, ResourceType};
        use rand::Rng;

        let mut rng = crate::core::dice::roll();
        let mut took_root: Vec<Position> = Vec::new();

        for agent in self.population.agents.iter_mut() {
            if !agent.state.is_alive {
                continue;
            }

            if agent.how_many_i_have("sproutedgrain") == 0 {
                continue;
            }

            if !rng.gen_bool(Self::WHAT_FALLS_OUT_OF_A_PACK) {
                continue;
            }

            agent.inventory.remove_item("sproutedgrain", 1);
            took_root.push(Position::new(
                agent.state.position.0,
                agent.state.position.1,
            ));
        }

        for where_it_fell in took_root {
            // Not on rock, not in a river, and not on top of something already
            // growing there
            let can_grow = self
                .world
                .grid
                .get_tile(&where_it_fell)
                .map(|tile| {
                    tile.terrain.can_be_tilled() || tile.terrain.is_cultivated()
                })
                .unwrap_or(false);

            if !can_grow {
                continue;
            }

            if self
                .world
                .resources
                .iter()
                .any(|resource| resource.position == where_it_fell)
            {
                continue;
            }

            let mut plant = ResourceNode::new(
                ResourceType::Grain,
                where_it_fell,
                Self::WHAT_ONE_SEED_COMES_TO,
            );
            plant.amount = 1;
            self.world.resources.push(plant);

            debug!("A dropped grain took root at {where_it_fell:?}");

            for agent in self
                .population
                .agents
                .iter_mut()
                .filter(|agent| agent.state.is_alive)
            {
                let apart = (agent.state.position.0 - where_it_fell.x).abs()
                    + (agent.state.position.1 - where_it_fell.y).abs();

                if apart <= Self::CLOSE_ENOUGH_TO_SEE_IT_COME_UP {
                    agent
                        .practices
                        .saw_it_work(crate::agents::practices::Practice::Farming);
                }
            }
        }
    }

    /// Leave on the ground what bodies have to leave.
    ///
    /// Everything a settlement grew used to leave the world for good: eaten
    /// and gone, spoiled and deleted, buried nowhere. The soil was a stock
    /// being mined with no return at all, and the only thing that ever put
    /// anything back was an agent who had learned to tip a spoiled basket onto
    /// a field. Traced over thirty thousand ticks, farmed ground went from
    /// 0.53 fertility to 0.03 and stayed there.
    ///
    /// What a body takes in mostly comes out again, and what a body is comes
    /// back when it stops. Neither is a free lunch - rot keeps three fifths of
    /// what it works on and loses the rest, so the loop turns and loses on
    /// every turn. And it lands where the agent is standing rather than where
    /// the crop grew, which is exactly why carting muck onto a field is worth
    /// an agent's time.
    pub(in crate::analytics) fn return_what_the_living_and_the_dead_leave(&mut self) {
        use crate::world::Position;

        // What the living have to pass
        let leavings: Vec<((i32, i32, i32), f32)> = self
            .population
            .agents
            .iter_mut()
            .filter(|agent| agent.state.is_alive)
            .map(|agent| (agent.state.position, agent.state.void_waste()))
            .filter(|(_, waste)| *waste > 0.0)
            .collect();

        for (position, waste) in leavings {
            // Not just litter: a midden also has a smell and seeds in it.
            let here = Position::new(position.0, position.1);
            self.world.grid.somebody_voided_on(&here, waste);
        }

        // And what the dead leave where they fell
        let bodies = std::mem::take(&mut self.population.bodies_where_they_fell);

        for (position, soft, bone) in bodies {
            let here = Position::new(position.0, position.1);
            if let Some(tile) = self.world.grid.get_tile_mut(&here) {
                tile.soil.add_leaf_litter(soft);
                tile.soil.add_woody_litter(bone);
            }

            // And it fouls the ground it fell on, which is the whole reason a
            // body is a thing you want to be away from. Until now a corpse was
            // a nutrient deposit and nothing else - agents walked over their
            // own dead with no more consequence than walking over leaf mould.
            self.world
                .grid
                .somebody_voided_on(&here, soft * Self::HOW_MUCH_OF_A_BODY_IS_FOULING);
        }

        self.what_the_dead_left_behind();
    }

    /// What share of what a body is left on the ground counts as fouling.
    ///
    /// A body is a great deal of soft matter and only some of it is the part
    /// that makes ground foul, so this is well under one. What it has to do
    /// is put a fresh corpse comfortably over `FOUL_ENOUGH_TO_WALK_AWAY_FROM`,
    /// so that people move off ground somebody died on and come back to it
    /// once it has broken down.
    pub(in crate::analytics) const HOW_MUCH_OF_A_BODY_IS_FOULING: f32 = 0.4;

    /// What a person was carrying stays where they fell.
    ///
    /// Everything a people makes used to go into the ground with whoever
    /// happened to be holding it: an axe was a thing that existed for exactly
    /// as long as its owner did. A pack falls where its owner does, and the
    /// next person along can pick it up - which is most of how a stone-age
    /// people ever accumulates anything at all.
    pub(in crate::analytics) fn what_the_dead_left_behind(&mut self) {
        use crate::world::Position;

        let left = std::mem::take(&mut self.population.what_the_dead_left);
        let now = self.current_tick;

        for (item, position) in left {
            self.world
                .somebody_left_this(item, Position::new(position.0, position.1), now);
        }
    }

    /// How far off something has to be before it stops being this agent's
    /// problem
    pub(in crate::analytics) const CLOSE_ENOUGH_TO_WORRY_ABOUT: i32 = 10;

    /// How far a frightened agent puts between itself and the thing
    ///
    /// Far enough to be out of the range at which it would appraise the thing
    /// again, or it runs one pace, looks round, and runs one pace again.
    pub(in crate::analytics) const FAR_ENOUGH_AWAY: i32 = Self::CLOSE_ENOUGH_TO_WORRY_ABOUT + 5;

    /// How far an angry agent will go to reach the thing it is angry at
    pub(in crate::analytics) const WITHIN_A_STEP_OR_TWO: i32 = 5;

    /// Living on a midden, or beside somebody's body.
    ///
    /// "Spending time near dead bodies or fresh waste" - and the two are one
    /// question here, because a corpse fouls the ground it falls on the same
    /// way a midden does. Agents already step off foul ground when they
    /// notice it; what was missing is any reason to, beyond distaste.
    ///
    /// Nothing here is certain and nothing is fast. Standing on fouled ground
    /// for one tick is almost always nothing; living on it is what tells.
    pub(in crate::analytics) fn what_the_ground_underfoot_does(&mut self) {
        use rand::Rng;

        if self.current_tick % Self::HOW_OFTEN_THE_GROUND_IS_ASKED != 0 {
            return;
        }

        let now = self.current_tick;
        let mut rng = crate::core::dice::roll();

        for agent in self.population.agents.iter_mut() {
            if !agent.state.is_alive || agent.is_ailing() {
                continue;
            }

            let here = crate::world::Position::new(agent.state.position.0, agent.state.position.1);
            let Some(tile) = self.world.grid.get_tile(&here) else {
                continue;
            };

            if !tile.soil.is_foul() {
                continue;
            }

            // How foul, as a share of as foul as ground gets.
            let how_bad = (tile.soil.fouling / crate::world::Soil::AS_FOUL_AS_IT_GETS)
                .clamp(0.0, 1.0);
            let odds = Self::HOW_OFTEN_FOUL_GROUND_TELLS * how_bad as f64;

            if rng.gen_bool(odds.clamp(0.0, 1.0)) {
                agent.taken_ill_with(
                    crate::agents::Agent::OFF_FOUL_GROUND,
                    0.25 + 0.35 * how_bad,
                    now,
                );
            }
        }
    }

    /// How often the ground under everybody is asked about.
    ///
    /// Once a day rather than every tick: this is a question about living
    /// somewhere, not about walking across it.
    pub(in crate::analytics) const HOW_OFTEN_THE_GROUND_IS_ASKED: u32 = crate::environment::seasons::TICKS_PER_DAY;

    /// And how often a day spent on the worst ground there is makes somebody
    /// ill.
    ///
    /// One day in twenty at the very worst, which over a season on a midden
    /// is most of a settlement and over a week is almost nobody. Fouling
    /// breaks down, so this is a pressure to move rather than a sentence.
    pub(in crate::analytics) const HOW_OFTEN_FOUL_GROUND_TELLS: f64 = 0.05;
}
