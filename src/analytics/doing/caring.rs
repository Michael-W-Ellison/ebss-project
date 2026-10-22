// src/analytics/doing/caring.rs
//! Looking after a wound: pressing on it, and binding it up (#351).
//!
//! One method per `Action` variant, called from the dispatcher in
//! [`super::execute_action`].

use super::super::Simulation;
use crate::core::DriveType;
use crate::environment::ActionResult;

impl Simulation {
    /// `Action::PressOnWound`: a hand pressed on a wound that is bleeding.
    ///
    /// Stage 0 of medicine, "pressure on wounds", and born knowledge: nobody
    /// has to be shown to press on a cut. One's own hand slows it to a quarter;
    /// somebody else's two hands stop it. Every wound pressed is a step
    /// towards noticing that a strip tied over it holds the pressure for you
    /// (`do:Bandage`).
    pub(in crate::analytics) fn pressing_on_a_wound(
        &mut self,
        agent_index: usize,
        who: Option<uuid::Uuid>,
    ) -> ActionResult {
        let patient = match self.the_patient(agent_index, who) {
            Ok(patient) => patient,
            Err(refused) => return refused,
        };
        if self.population.agents[patient].state.bleeding <= 0.0 {
            return ActionResult::failure("It is not bleeding".to_string());
        }

        let bleeding = &mut self.population.agents[patient].state.bleeding;
        *bleeding = if patient == agent_index { *bleeding * Self::WHAT_ONES_OWN_HAND_LEAVES } else { 0.0 };
        if *bleeding < crate::agents::Agent::A_TRICKLE {
            *bleeding = 0.0;
        }

        let carer = &mut self.population.agents[agent_index];
        carer.skills.made_one(Self::A_WOUND_PRESSED);
        if carer.skills.times_made(Self::A_WOUND_PRESSED) >= Self::WOUNDS_PRESSED_BEFORE_A_DRESSING {
            carer.found_out_how_to("do:Bandage");
        }

        ActionResult::success()
            .with_drive_change(if who.is_some() { DriveType::Protection } else { DriveType::Rest }, -0.2)
            .with_energy_cost(1.0)
            .with_message("Pressed on the wound".to_string())
    }

    /// `Action::Bandage`: a strip of bark, sinew or hide tied over an open
    /// wound (#351).
    ///
    /// Stage 1, "bandaging", found out by pressing on wounds. It stops the
    /// bleeding as two hands would, and keeps on doing it: dressed, a wound
    /// closes twice as fast and turns half as often, for a week.
    pub(in crate::analytics) fn bandaging(
        &mut self,
        agent_index: usize,
        who: Option<uuid::Uuid>,
        turn_now: u32,
    ) -> ActionResult {
        let patient = match self.the_patient(agent_index, who) {
            Ok(patient) => patient,
            Err(refused) => return refused,
        };
        if self.population.agents[patient].state.an_open_wound <= 0.0 {
            return ActionResult::failure("There is no wound to dress".to_string());
        }
        if turn_now < self.population.agents[patient].state.wound_dressed_until {
            return ActionResult::failure("It is dressed already".to_string());
        }

        let carer = &mut self.population.agents[agent_index];
        let holding = |what: &str| carer.how_many_i_have(what);
        let Ok(using) = crate::environment::tags::what_would_be_used(Self::WHAT_A_DRESSING_TAKES, &holding) else {
            return ActionResult::failure("Nothing to bind it with".to_string());
        };
        for (what, how_many) in using {
            carer.inventory.remove_item(what, how_many);
        }

        let state = &mut self.population.agents[patient].state;
        state.bleeding = 0.0;
        state.wound_dressed_until = turn_now + Self::A_DRESSING_IS_GOOD_FOR;

        ActionResult::success()
            .with_drive_change(if who.is_some() { DriveType::Protection } else { DriveType::Rest }, -0.3)
            .with_energy_cost(1.0)
            .with_message("Bound up the wound".to_string())
    }

    /// Who is being looked after: oneself, or somebody within reach of a hand.
    fn the_patient(&self, agent_index: usize, who: Option<uuid::Uuid>) -> Result<usize, ActionResult> {
        let Some(id) = who else {
            return Ok(agent_index);
        };
        let here = self.population.agents[agent_index].state.position;
        let found = self
            .population
            .agents
            .iter()
            .position(|other| other.id == id && other.state.is_alive)
            .ok_or_else(|| ActionResult::failure("Nobody of that name here".to_string()))?;
        let there = self.population.agents[found].state.position;
        if (here.0 - there.0).abs().max((here.1 - there.1).abs()) > Self::WITHIN_A_HANDS_REACH {
            return Err(ActionResult::failure("Too far off to reach the wound".to_string()));
        }
        Ok(found)
    }

    /// How close a carer has to be to put a hand on a wound.
    pub(in crate::analytics) const WITHIN_A_HANDS_REACH: i32 = 1;

    /// What of its bleeding a wound keeps under its owner's own hand.
    pub(in crate::analytics) const WHAT_ONES_OWN_HAND_LEAVES: f32 = 0.25;

    /// What the practice record calls a wound pressed on, and how many before
    /// somebody ties something over one instead (#351).
    pub(in crate::analytics) const A_WOUND_PRESSED: &'static str = "woundpressed";
    pub(in crate::analytics) const WOUNDS_PRESSED_BEFORE_A_DRESSING: u32 = 4;

    /// What a dressing takes: a strip of something that binds.
    pub(in crate::analytics) const WHAT_A_DRESSING_TAKES: &'static [crate::environment::tags::Wanted] =
        &[crate::environment::tags::Wanted { is: crate::environment::tags::Tag::Cordage, how_much: 1 }];

    /// How long a dressing is good for, in ticks: a week.
    pub(in crate::analytics) const A_DRESSING_IS_GOOD_FOR: u32 = 7 * crate::environment::seasons::TICKS_PER_DAY;
}
