// src/analytics/doing/ditching.rs
//! Water diversion on a very small scale (ISSUES_FOUND #383).
//!
//! "Water diversion on a very small scale", stage 1 of the agricultural
//! path. Most ground in this world holds as much water as a plant does best
//! on whatever the sky does; hills, mountain, desert and salt flat do not
//! (`ResourceNode::DRIER_THAN_A_PLANT_LIKES`). A ditch scratched from a stream
//! or a spring to a patch of wild stands on such ground waters them
//! (`ResourceNode::watered`), and silts up over half a year.
//!
//! Somebody who gathers often enough from the stands near home on dry ground
//! with water close by thinks to lead the water to them, or sees it done.
//! Somebody who knows how picks the patch near home most in need of it, in
//! the growing season, walks to it and digs.

use super::super::Simulation;
use crate::core::DriveType;
use crate::environment::seasons::Season;
use crate::environment::{Action, ActionResult};
use crate::world::{Position, ResourceNode};

impl Simulation {
    /// How far a ditch dug by hand is led from the water, cells.
    pub(in crate::analytics) const HOW_FAR_A_DITCH_RUNS: u32 = 4;

    /// How far round its end a ditch waters, cells.
    pub(in crate::analytics) const HOW_FAR_A_DITCH_WATERS: u32 = 2;

    /// How few stands on dry ground make a patch worth a ditch.
    pub(in crate::analytics) const A_PATCH_WORTH_A_DITCH: usize = 2;

    /// A stand watered at least this well needs no ditch dug to it yet.
    pub(in crate::analytics) const WATERED_ENOUGH: f32 = 0.5;

    /// How long a camp leaves between one ditch and the next, days.
    pub(in crate::analytics) const DAYS_BETWEEN_A_CAMPS_DITCHES: u32 = 7;

    /// What practice records call gathering from a stand near home on ground
    /// drier than it likes, with water near enough to lead to it, and how
    /// often before somebody thinks to.
    pub(in crate::analytics) const GATHERED_DRY_BY_WATER: &'static str = "gathereddrybywater";
    pub(in crate::analytics) const GATHERED_DRY_BY_WATER_BEFORE_DITCHING: u32 = 40;

    /// Whether this stand is one a ditch would do something for: a wild food
    /// stand on ground drier than a plant likes, with water near enough to
    /// lead to it.
    pub(in crate::analytics) fn would_a_ditch_water_this(&self, node: &ResourceNode) -> bool {
        node.is_a_wild_food_stand()
            && self.world.how_wet_the_ground_is_of_itself(&node.position) < ResourceNode::DRIER_THAN_A_PLANT_LIKES
            && self.world.is_there_water_to_lead_from(node.position, Self::HOW_FAR_A_DITCH_RUNS)
    }

    /// The stands a ditch ended at `at` would water, and are not watered
    /// well enough already.
    fn what_a_ditch_here_would_water(&self, at: Position) -> Vec<usize> {
        let reach = Self::HOW_FAR_A_DITCH_WATERS as i32;
        self.world
            .node_numbers_near(at, Self::HOW_FAR_A_DITCH_WATERS)
            .into_iter()
            .filter(|&number| {
                let node = &self.world.resources[number];
                let (dx, dy) = (node.position.x - at.x, node.position.y - at.y);
                dx * dx + dy * dy <= reach * reach
                    && node.is_a_wild_food_stand()
                    && node.watered < Self::WATERED_ENOUGH
                    && self.world.how_wet_the_ground_is_of_itself(&node.position)
                        < ResourceNode::DRIER_THAN_A_PLANT_LIKES
            })
            .collect()
    }

    /// The patch near this home most in need of a ditch, if any is: where it
    /// would end. Within a forage, with water to lead from, and not when the
    /// camp dug one within the week.
    pub(in crate::analytics) fn the_ground_worth_a_ditch(&self, home: (i32, i32)) -> Option<Position> {
        if self
            .world
            .days_since_this_camp_dug_a_ditch(home)
            .is_some_and(|days| days < Self::DAYS_BETWEEN_A_CAMPS_DITCHES)
        {
            return None;
        }
        let mut best: Option<(usize, Position)> = None;
        for number in self.world.node_numbers_near(Position::new(home.0, home.1), Self::FORAGE_RADIUS) {
            let node = &self.world.resources[number];
            if node.watered >= Self::WATERED_ENOUGH || !self.would_a_ditch_water_this(node) {
                continue;
            }
            let watered = self.what_a_ditch_here_would_water(node.position).len();
            if watered >= Self::A_PATCH_WORTH_A_DITCH && best.is_none_or(|(most, _)| watered > most) {
                best = Some((watered, node.position));
            }
        }
        best.map(|(_, at)| at)
    }

    /// The next of watering a patch near home (#383): dug where this one
    /// stands if it is the patch, else walked to. Only by somebody grown who
    /// knows how, with nothing pressing, while things are growing: the
    /// spring and the summer.
    pub(in crate::analytics) fn a_ditch_to_dig(
        &self,
        agent: &crate::agents::Agent,
        here: (i32, i32, i32),
    ) -> Option<Action> {
        if agent.state.years_old() < crate::agents::Agent::OLD_ENOUGH_TO_KEEP_A_KIT
            || !agent.knows_how_to_do("DigDitch")
            || !agent.immediate_needs_met()
            || !matches!(self.world.climate.current_season(), Season::Spring | Season::Summer)
        {
            return None;
        }
        let patch = self.the_ground_worth_a_ditch(Self::where_this_one_lives(agent))?;
        if Self::within((here.0, here.1), (patch.x, patch.y), 1) {
            Some(Action::DigDitch)
        } else {
            Some(Action::Move { target: (patch.x, patch.y, here.2) })
        }
    }

    /// `Action::DigDitch`: a ditch led from water near where this one
    /// stands, watering every stand on dry ground about its end.
    pub(in crate::analytics) fn digging_a_ditch(&mut self, agent_index: usize) -> ActionResult {
        let agent = &self.population.agents[agent_index];
        if !agent.knows_how_to_do("DigDitch") {
            return ActionResult::failure("Nobody here has led water to a stand".to_string());
        }
        let here = Position::new(agent.state.position.0, agent.state.position.1);
        let home = Self::where_this_one_lives(agent);
        if !self.world.is_there_water_to_lead_from(here, Self::HOW_FAR_A_DITCH_RUNS) {
            return ActionResult::failure("No water near enough to lead here".to_string());
        }
        let watering = self.what_a_ditch_here_would_water(here);
        if watering.is_empty() {
            return ActionResult::failure("Nothing here a ditch would water".to_string());
        }
        for &number in &watering {
            self.world.resources[number].watered = 1.0;
        }
        self.world.ditches_dug += 1;
        self.world.this_camp_dug_a_ditch(home);

        ActionResult::success()
            .with_drive_change(DriveType::Sustenance, -0.2)
            .with_energy_cost(8.0)
            .with_message(format!("Led water to {} stands", watering.len()))
    }
}
