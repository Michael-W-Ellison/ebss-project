// src/analytics/doing/mod.rs
//! Doing the thing: one module per family of verbs, and the dispatcher that
//! chooses between them.
//!
//! `execute_action` was a single function of five thousand seven hundred
//! lines - a third of `analytics/mod.rs` - holding a fifty-two arm `match`.
//! Every arm was reachable only by scrolling, no two could be read side by
//! side, and a change to one meant a diff nobody could review against the
//! other fifty-one.
//!
//! It is a dispatcher now. The arms are methods, grouped by what they are
//! about rather than by the order somebody happened to add them:
//!
//! - [`eating`] - food into a body, and the keeping of it
//! - [`getting`] - taking what the country has
//! - [`making`] - one thing turned into another
//! - [`ground`] - working the soil
//! - [`keeping`] - carrying and putting by
//! - [`meeting`] - what passes between two people
//! - [`fighting`] - threat, and the answers to it
//! - [`moving`] - going somewhere, or staying put
//! - [`looking`] - attending to a thing
//!
//! Nothing about what the model does changed here: the bodies moved verbatim,
//! and three seeds run for six hundred ticks give byte-identical worlds either
//! side of the move. That check is only possible because of the work in
//! ISSUES_FOUND.md #94, and it is what makes a refactor of this size a
//! reviewable change rather than an act of faith.

pub mod eating;
pub mod fighting;
pub mod getting;
pub mod ground;
pub mod keeping;
pub mod looking;
pub mod making;
pub mod meeting;
pub mod moving;

use super::Simulation;
use crate::environment::{Action, ActionResult};

impl Simulation {
    /// Do the thing the agent has decided on, and say what came of it.
    ///
    /// Two things happen before the verb does: one roll, drawn here and lent
    /// to whichever verb needs it - so that the number of draws a turn takes
    /// does not depend on which arm is chosen - and one check against the verb
    /// matrix for what these hands are short of, in one place rather than in
    /// thirty arms each deciding for itself whether a man needs a knife to
    /// skin something. See `environment::verbs`.
    pub(crate) fn execute_action(&mut self, action: &Action, agent_index: usize) -> ActionResult {
        let mut rng = crate::core::dice::roll();

        // Doing the work is what keeps a hand in it - see
        // `Skills::let_unused_skills_rust`
        let tick_now = self.current_tick;

        if let Some(missing) = self.what_these_hands_are_short_of(action, agent_index) {
            return ActionResult::failure(missing);
        }

        match action {
            Action::Eat { food_type } => self.eating(food_type, agent_index, &mut rng),
            Action::Sleep { duration } => self.sleeping(duration, agent_index),
            Action::Gather { resource_type } => self.gathering(resource_type, agent_index, &mut rng, tick_now),
            Action::Build { structure_type, position } => self.building(structure_type, position, agent_index, tick_now),
            Action::Attack { target_agent_id, weapon } => self.attacking(target_agent_id, weapon, agent_index, &mut rng, tick_now),
            Action::Craft { item_type } => self.crafting(item_type, agent_index, tick_now),
            Action::Move { target } => self.walking(target, agent_index),
            Action::Store { item_type, amount } => self.storing(item_type, amount, agent_index),
            Action::Retrieve { item_type, amount } => self.retrieving(item_type, amount, agent_index),
            Action::Hunt { animal_id, weapon } => self.hunting(animal_id, weapon, agent_index, &mut rng, tick_now),
            Action::Fight { animal_id, weapon } => self.fighting_a_beast(animal_id, weapon, agent_index, &mut rng, tick_now),
            Action::Tame { animal_id, food_type } => self.taming(animal_id, food_type, agent_index, tick_now),
            Action::CollectAnimalProduct { animal_id } => self.collecting_from_a_beast(animal_id, agent_index, tick_now),
            Action::HarvestPlant { plant_id } => self.harvesting_a_plant(plant_id, agent_index, &mut rng, tick_now),
            Action::SeekShelter => self.seeking_shelter(agent_index),
            Action::Socialize { target_agent_id } => self.socialising(target_agent_id, agent_index, &mut rng, tick_now),
            Action::ShareInformation { target_agent_id } => self.sharing_information(target_agent_id, agent_index, &mut rng),
            Action::Mate { target_agent_id } => self.mating(target_agent_id, agent_index, &mut rng),
            Action::Mount { transport_id } => self.mounting(transport_id, agent_index),
            Action::Dismount => self.dismounting(agent_index),
            Action::LightFire => self.lighting_a_fire(agent_index),
            Action::Cook { food_type } => self.cooking(food_type, agent_index, &mut rng),
            Action::MakeClothing { garment } => self.making_clothing(garment, agent_index, tick_now),
            Action::WearClothing { garment } => self.wearing_clothing(garment, agent_index),
            Action::TillSoil => self.tilling_soil(agent_index, tick_now),
            Action::TrySwapping { instead_of_making, instead_of, put_in } => self.trying_a_swap(instead_of_making, instead_of, put_in, agent_index, tick_now),
            Action::TakeFrom { from } => self.taking_from(from, agent_index, tick_now),
            Action::Equip { what } => self.equipping(what, agent_index),
            Action::Unequip { what } => self.unequipping(what, agent_index),
            Action::Freeze => self.freezing(agent_index),
            Action::FleeFrom { away_from } => self.fleeing_from(away_from, agent_index),
            Action::Examine { what } => self.examining(what, agent_index, &mut rng, tick_now),
            Action::Boil => self.boiling(agent_index),
            Action::Salt { what } => self.salting(what, agent_index),
            Action::Dry { what } => self.drying(what, agent_index, tick_now),
            Action::Excavate => self.excavating(agent_index, tick_now),
            Action::Cover { what } => self.covering(what, agent_index, tick_now),
            Action::PickUp { what } => self.picking_up(what, agent_index, tick_now),
            Action::PutDown { what } => self.putting_down(what, agent_index, tick_now),
            Action::AskAbout { who, what } => self.asking_about(who, what, agent_index),
            Action::Trade { with } => self.trading(with, agent_index, tick_now),
            Action::GoWithout { for_them } => self.going_without(for_them, agent_index),
            Action::GiveTo { to } => self.giving_to(to, agent_index, tick_now),
            Action::Work { verb, to } => self.working(verb, to, agent_index, &mut rng, tick_now),
            Action::Taste => self.tasting(agent_index, &mut rng),
            Action::TakeCutting => self.taking_a_cutting(agent_index, tick_now),
            Action::PlantCutting => self.planting_a_cutting(agent_index, tick_now),
            Action::TendField => self.tending_a_field(agent_index, tick_now),
            Action::SpreadMuck => self.spreading_muck(agent_index, tick_now),
            Action::Fish => self.fishing(agent_index, &mut rng, tick_now),
            Action::SetSnare => self.setting_a_snare(agent_index, tick_now),
            Action::CheckSnares => self.going_round_the_line(agent_index, tick_now),
            Action::Wait => self.waiting(agent_index, &mut rng),
            Action::Explore { direction } => self.exploring(direction, agent_index, tick_now),
        }
    }
}
