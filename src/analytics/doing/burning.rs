// src/analytics/doing/burning.rs
//! Controlled burning (ISSUES_FOUND #381).
//!
//! "Controlled burning", stage 1 of the agricultural path. Grass and brush
//! burnt off in the dry season come back as a flush of young shoots, seed and
//! fruit, which carries more and comes back faster for a year
//! (`ResourceNode::burnt`). The country burns of itself now and then, lit by
//! lightning (`World::the_dry_country_burns`), and somebody who gathers often
//! enough on ground a fire went over finds out that it can be done on purpose.
//!
//! Somebody who knows how picks a patch near home, off the camp's own ground,
//! where what stands now is less than what the flush would add, walks to it
//! with a brand from the camp's fire, and sets it alight.

use super::super::Simulation;
use crate::core::DriveType;
use crate::environment::{Action, ActionResult};
use crate::world::{Position, ResourceNode, World};

impl Simulation {
    /// How far a fire set on purpose runs, cells: a patch, burnt on a still
    /// day and let go out against what was burnt before.
    pub(in crate::analytics) const HOW_FAR_A_BURN_GOES: u32 = 3;

    /// How few stands that burn make a patch worth setting alight.
    pub(in crate::analytics) const A_PATCH_WORTH_THE_FIRE: usize = 3;

    /// Ground burnt so lately that what came up is still more than half its
    /// fresh flush is not burnt again: about half a year.
    pub(in crate::analytics) const STILL_COMING_ON_FROM_A_FIRE: f32 = 0.5;

    /// How long a camp leaves between one burn and the next, days: the fire
    /// is watched out before another is set.
    pub(in crate::analytics) const DAYS_BETWEEN_A_CAMPS_BURNS: f32 = 14.0;

    /// Ground a fire went over within the year, which anybody gathering there
    /// can see by the char on the old stems: what a summer's fire burnt still
    /// shows when the stands bear again the next summer.
    pub(in crate::analytics) const BURNT_WITHIN_THE_YEAR: f32 = 0.05;

    /// What practice records call gathering on ground a fire went over within
    /// the year, and how often before somebody thinks to set the fire
    /// themselves.
    pub(in crate::analytics) const GATHERED_ON_BURNT_GROUND: &'static str = "gatheredonburntground";
    pub(in crate::analytics) const BURNT_GROUND_GATHERED_BEFORE_BURNING: u32 = 12;

    /// What a fire would do for the stands within `reach` of `at`, against
    /// what it would burn off: what the flush is worth over its year, less
    /// what stands now. `None` where too few stands burn or the ground is
    /// still coming on from the last fire.
    ///
    /// The flush is worth what the stand carries, by what a burn brings on,
    /// halved for fading over its year. So ground stripped bare, or out of
    /// its season with nothing on it, is always worth burning; a patch hung
    /// with fruit is not.
    pub(in crate::analytics) fn what_a_burn_here_comes_to(&self, at: Position, reach: u32) -> Option<f32> {
        let reach_squared = (reach as i64) * (reach as i64);
        let mut stands = 0;
        let mut lost = 0.0;
        let mut added = 0.0;
        for number in self.world.node_numbers_near(at, reach) {
            let node = &self.world.resources[number];
            let (dx, dy) = ((node.position.x - at.x) as i64, (node.position.y - at.y) as i64);
            if dx * dx + dy * dy > reach_squared || !node.is_brought_on_by_fire() {
                continue;
            }
            if node.burnt > Self::STILL_COMING_ON_FROM_A_FIRE {
                return None;
            }
            let yields = self.world.grid.what_it_yields_here_for(&node.position, node.resource_type);
            let carries = node.how_heavy_a_crop_it_carries(yields) as f32 / node.what_the_fire_did();
            stands += 1;
            lost += node.amount as f32;
            added += carries * ResourceNode::WHAT_A_BURN_BRINGS_ON * 0.5;
        }
        (stands >= Self::A_PATCH_WORTH_THE_FIRE).then_some(added - lost)
    }

    /// Whether a fire set at `at` would reach the camp's own ground at `home`.
    pub(in crate::analytics) fn would_it_reach_the_camp(at: Position, home: (i32, i32)) -> bool {
        Self::within((at.x, at.y), home, Self::ON_THE_CAMPS_GROUND + Self::HOW_FAR_A_BURN_GOES as i32)
    }

    /// The patch near this home most worth burning, if any is worth it: the
    /// middle of it. Only off the camp's own ground, within a forage, and not
    /// where the camp burnt within the last fortnight.
    pub(in crate::analytics) fn the_ground_worth_burning(&self, home: (i32, i32)) -> Option<Position> {
        let near_home = self
            .world
            .node_numbers_near(Position::new(home.0, home.1), Self::FORAGE_RADIUS);
        let lately = 1.0 - Self::DAYS_BETWEEN_A_CAMPS_BURNS * ResourceNode::HOW_FAST_A_BURN_FADES_A_DAY;
        let mut best: Option<(f32, Position)> = None;
        for &number in &near_home {
            let node = &self.world.resources[number];
            if !node.is_brought_on_by_fire() {
                continue;
            }
            let at = (node.position.x, node.position.y);
            if !Self::within(at, home, Self::FORAGE_RADIUS as i32) {
                continue;
            }
            if node.burnt > lately {
                return None;
            }
            if Self::would_it_reach_the_camp(node.position, home) {
                continue;
            }
            let Some(worth) = self.what_a_burn_here_comes_to(node.position, Self::HOW_FAR_A_BURN_GOES) else {
                continue;
            };
            if worth > 0.0 && best.is_none_or(|(most, _)| worth > most) {
                best = Some((worth, node.position));
            }
        }
        best.map(|(_, at)| at)
    }

    /// The next of burning off a patch near home (#381): set alight where
    /// this one stands if it is the patch, else walked to. Only by somebody
    /// grown who knows how, with nothing pressing, on a day for burning, with
    /// a fire burning at the camp to take a brand from.
    pub(in crate::analytics) fn a_patch_to_burn(
        &self,
        agent: &crate::agents::Agent,
        here: (i32, i32, i32),
    ) -> Option<Action> {
        if agent.state.years_old() < crate::agents::Agent::OLD_ENOUGH_TO_KEEP_A_KIT
            || !agent.knows_how_to_do("BurnOff")
            || !agent.immediate_needs_met()
        {
            return None;
        }
        let precipitation = self.world.climate.weather.weather_type.precipitation_intensity();
        if !World::is_it_a_day_for_burning(self.world.climate.current_season(), precipitation) {
            return None;
        }
        let home = Self::where_this_one_lives(agent);
        self.nearest_fire_from((home.0, home.1, 0), Self::NEAR_ENOUGH_TO_SLEEP_WITH_THE_OTHERS, true)?;
        let patch = self.the_ground_worth_burning(home)?;
        if Self::within((here.0, here.1), (patch.x, patch.y), 1) {
            Some(Action::BurnOff)
        } else {
            Some(Action::Move { target: (patch.x, patch.y, here.2) })
        }
    }

    /// `Action::BurnOff`: the ground about where this one stands set alight,
    /// and every stand on it burnt off for the flush to come. Not on a wet
    /// day or out of the dry seasons, and not near enough the camp for the
    /// fire to reach it.
    pub(in crate::analytics) fn burning_off(&mut self, agent_index: usize) -> ActionResult {
        let agent = &self.population.agents[agent_index];
        if !agent.knows_how_to_do("BurnOff") {
            return ActionResult::failure("Nobody here has burnt the country off".to_string());
        }
        let precipitation = self.world.climate.weather.weather_type.precipitation_intensity();
        if !World::is_it_a_day_for_burning(self.world.climate.current_season(), precipitation) {
            return ActionResult::failure("Too wet to burn".to_string());
        }
        let here = Position::new(agent.state.position.0, agent.state.position.1);
        if Self::would_it_reach_the_camp(here, Self::where_this_one_lives(agent)) {
            return ActionResult::failure("Too near the camp to set a fire".to_string());
        }
        let burnt = self.world.burn_over(here, Self::HOW_FAR_A_BURN_GOES);
        if burnt == 0 {
            return ActionResult::failure("Nothing here that a fire would bring on".to_string());
        }
        self.world.burns_set += 1;

        ActionResult::success()
            .with_drive_change(DriveType::Sustenance, -0.2)
            .with_energy_cost(4.0)
            .with_message(format!("Burnt off {burnt} stands"))
    }
}
