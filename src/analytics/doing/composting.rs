// src/analytics/doing/composting.rs
//! Compost from the midden (ISSUES_FOUND #386).
//!
//! "Composting and midden enrichment", stage 2 of the agricultural path. What
//! goes onto a camp's midden - spoiled food, offal, rotten hides, what a body
//! leaves - rots down over a couple of months (`Midden::holds`,
//! `Midden::rotted`). A gardener digs out what has rotted and carries it to
//! the field, where it goes on as muck does (`Action::SpreadMuck`). Muck was
//! fold dung and what went off in a pack; the midden kept everything else.

use super::super::Simulation;
use crate::core::DriveType;
use crate::environment::{Action, ActionResult};
use crate::world::{Midden, Position};

impl Simulation {
    /// What a load of compost is called in a pack.
    pub(in crate::analytics) const COMPOST: &'static str = "compost";

    /// Whether the camp at `home` has a garden near it: a crop standing on
    /// broken ground within a forage.
    pub(in crate::analytics) fn does_this_camp_keep_a_garden(&self, home: (i32, i32)) -> bool {
        self.world
            .node_numbers_near(Position::new(home.0, home.1), Self::FORAGE_RADIUS)
            .into_iter()
            .any(|number| self.world.resources[number].on_a_field)
    }

    /// The next of carrying compost to the garden (#386): dug out where this
    /// one stands if it is the camp's midden, else walked to. Only by
    /// somebody grown who knows how, with nothing pressing and no compost
    /// already in the pack, at a camp with a garden and a midden with a load
    /// rotted down in it.
    pub(in crate::analytics) fn compost_to_dig(
        &self,
        agent: &crate::agents::Agent,
        here: (i32, i32, i32),
    ) -> Option<Action> {
        if agent.state.years_old() < crate::agents::Agent::OLD_ENOUGH_TO_KEEP_A_KIT
            || !agent.knows_how_to_do("DigOutMidden")
            || !agent.immediate_needs_met()
            || agent.how_many_i_have(Self::COMPOST) > 0
        {
            return None;
        }
        let home = Self::where_this_one_lives(agent);
        let midden = &self.world.middens[self.world.midden_serving(home)?];
        if midden.rotted < Midden::WHAT_A_LOAD_OF_COMPOST_IS || !self.does_this_camp_keep_a_garden(home) {
            return None;
        }
        if Self::within((here.0, here.1), (midden.at.x, midden.at.y), 1) {
            Some(Action::DigOutMidden)
        } else {
            Some(Action::Move { target: (midden.at.x, midden.at.y, here.2) })
        }
    }

    /// `Action::DigOutMidden`: what has rotted down on the camp's midden dug
    /// out, as many loads as one trip carries.
    pub(in crate::analytics) fn digging_out_the_midden(&mut self, agent_index: usize) -> ActionResult {
        let agent = &self.population.agents[agent_index];
        if !agent.knows_how_to_do("DigOutMidden") {
            return ActionResult::failure("Nobody here puts the midden on a garden".to_string());
        }
        let home = Self::where_this_one_lives(agent);
        let here = (agent.state.position.0, agent.state.position.1);
        let Some(which) = self.world.midden_serving(home) else {
            return ActionResult::failure("The camp keeps no midden".to_string());
        };
        let midden = &mut self.world.middens[which];
        if !Self::within(here, (midden.at.x, midden.at.y), 1) {
            return ActionResult::failure("Not at the midden".to_string());
        }
        let loads = ((midden.rotted / Midden::WHAT_A_LOAD_OF_COMPOST_IS) as u32).min(Midden::LOADS_DUG_OUT_AT_ONCE);
        if loads == 0 {
            return ActionResult::failure("Nothing rotted down yet".to_string());
        }
        midden.rotted -= loads as f32 * Midden::WHAT_A_LOAD_OF_COMPOST_IS;
        self.world.compost_dug += loads as u64;
        self.population.agents[agent_index]
            .inventory
            .add_item(crate::agents::InventoryItem::new_with_weight(Self::COMPOST.to_string(), loads, 1.0));

        ActionResult::success()
            .with_drive_change(DriveType::Sustenance, -0.1)
            .with_energy_cost(6.0)
            .with_message(format!("Dug {loads} loads of compost out of the midden"))
    }
}
