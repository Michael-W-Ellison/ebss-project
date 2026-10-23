// src/analytics/doing/sanitation.rs
//! A camp's refuse put somewhere (#357): a midden set aside away from where
//! people sleep and draw water, and a latrine pit beside it.
//!
//! One method per `Action` variant, called from the dispatcher in
//! [`super::execute_action`].

use super::super::Simulation;
use crate::environment::ActionResult;
use crate::world::{Latrine, Midden, Position};

impl Simulation {
    /// Where a camp living at `home` puts its midden: of the ground a
    /// midden's distance off in eight directions, the open ground that is
    /// furthest from water, and not foul already.
    pub(in crate::analytics) fn where_the_midden_goes(&self, home: (i32, i32)) -> Option<Position> {
        let far = Midden::HOW_FAR_FROM_THE_HEARTH;
        [(1, 0), (1, 1), (0, 1), (-1, 1), (-1, 0), (-1, -1), (0, -1), (1, -1)]
            .iter()
            .map(|(dx, dy)| Position::new(home.0 + dx * far, home.1 + dy * far))
            .filter(|there| {
                self.world.grid.get_tile(there).is_some_and(|tile| {
                    tile.terrain.is_walkable() && !crate::environment::fauna::what_this_ground_offers(tile.terrain.terrain_type).is_water && !tile.soil.is_foul()
                })
            })
            .max_by_key(|there| (self.how_far_from_water(*there), there.x, there.y))
    }

    /// How far a place is from the nearest open water, up to a midden's
    /// distance and a half.
    fn how_far_from_water(&self, at: Position) -> i32 {
        let looked = Midden::HOW_FAR_FROM_THE_HEARTH * 3 / 2;
        for reach in 0..=looked {
            for dx in -reach..=reach {
                for dy in -reach..=reach {
                    if dx.abs().max(dy.abs()) != reach {
                        continue;
                    }
                    if self
                        .world
                        .grid
                        .get_tile(&Position::new(at.x + dx, at.y + dy))
                        .is_some_and(|tile| crate::environment::fauna::what_this_ground_offers(tile.terrain.terrain_type).is_water)
                    {
                        return reach;
                    }
                }
            }
        }
        looked + 1
    }

    /// `Action::StartMidden`: a place set aside for the camp's refuse
    /// (#357), by somebody who has been laid up by filth and seen why.
    pub(in crate::analytics) fn starting_a_midden(&mut self, agent_index: usize, turn_now: u32) -> ActionResult {
        let agent = &self.population.agents[agent_index];
        if !agent.knows_how_to_do("StartMidden") {
            return ActionResult::failure("Nobody here keeps the refuse in one place".to_string());
        }
        let home = Self::where_this_one_lives(agent);
        let home = (home.0, home.1);
        if self.world.midden_serving(home).is_some() {
            return ActionResult::failure("The camp has a midden".to_string());
        }
        let Some(there) = self.where_the_midden_goes(home) else {
            return ActionResult::failure("Nowhere clean to put a midden".to_string());
        };
        self.world.middens.push(Midden { at: there, started: turn_now });
        ActionResult::success()
            .with_energy_cost(3.0)
            .with_message(format!("Set aside a midden at {:?}", (there.x, there.y)))
    }

    /// `Action::DigLatrine`: a pit beside the midden to take the camp's
    /// leavings under the ground (#357), by a people that has settled.
    pub(in crate::analytics) fn digging_a_latrine(&mut self, agent_index: usize, turn_now: u32) -> ActionResult {
        let agent = &self.population.agents[agent_index];
        if !agent.knows_how_to_do("DigLatrine") {
            return ActionResult::failure("Nobody here digs a latrine".to_string());
        }
        let home = Self::where_this_one_lives(agent);
        let home = (home.0, home.1);
        if self.world.latrine_serving(home).is_some() {
            return ActionResult::failure("The camp has a latrine with room in it".to_string());
        }
        let Some(midden) = self.world.midden_serving(home) else {
            return ActionResult::failure("A latrine goes by the midden".to_string());
        };
        let at = self.world.middens[midden].at;
        let beside = Position::new(at.x + 1, at.y);
        if !self.is_ground_a_pit_will_go_in(beside) {
            return ActionResult::failure("No ground by the midden a pit will go in".to_string());
        }
        self.world.latrines.retain(|latrine| latrine.holds < Latrine::WHAT_IT_TAKES);
        self.world.latrines.push(Latrine { at: beside, dug: turn_now, holds: 0.0 });
        self.population.agents[agent_index]
            .skills
            .practise(crate::agents::SkillType::Construction, 3, turn_now);
        ActionResult::success()
            .with_energy_cost(10.0)
            .with_message("Dug a latrine by the midden".to_string())
    }
}
