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

        let bled_heavily = self.population.agents[patient].state.bleeding;
        self.a_wound_seen_by_the_fire(agent_index, bled_heavily);
        let bleeding = &mut self.population.agents[patient].state.bleeding;
        *bleeding = if patient == agent_index { *bleeding * Self::WHAT_ONES_OWN_HAND_LEAVES } else { 0.0 };
        if *bleeding < crate::agents::Agent::A_TRICKLE {
            *bleeding = 0.0;
        }

        let carer = &mut self.population.agents[agent_index];
        carer.skills.made_one(Self::A_WOUND_PRESSED);
        if carer.skills.times_made(Self::A_WOUND_PRESSED) >= Self::WOUNDS_PRESSED_BEFORE_A_DRESSING {
            carer.it_comes_to_me("do:Bandage", Self::WOUNDS_PRESSED_BEFORE_A_DRESSING);
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

        // A herb bound in under it, by somebody who knows to (#404)
        let poulticed = carer.knows_the_way("poultices_and_teas") && {
            let herb = carer
                .inventory
                .get_all_items()
                .iter()
                .filter(|(id, item)| item.quantity > 0 && crate::environment::remedies::is_a_remedy(id))
                .max_by_key(|(id, _)| {
                    crate::environment::remedies::what_this_is_good_for(id)
                        .is_some_and(|remedy| remedy.eases == crate::environment::remedies::WhatARemedyEases::TheSkin)
                })
                .map(|(id, _)| id.clone());
            herb.is_some_and(|herb| carer.inventory.remove_item(&herb, 1).is_some())
        };
        // A point to hand while binding one is how sewing it comes to mind
        // (#401)
        if Self::has_something_to_pierce_with(carer) {
            carer.skills.made_one(Self::A_WOUND_BOUND_WITH_A_POINT_TO_HAND);
            if carer.skills.times_made(Self::A_WOUND_BOUND_WITH_A_POINT_TO_HAND) >= Self::WOUNDS_BOUND_BEFORE_SEWING_ONE {
                carer.it_comes_to_me("do:StitchWound", Self::WOUNDS_BOUND_BEFORE_SEWING_ONE);
            }
        }

        let bled_heavily = self.population.agents[patient].state.bleeding;
        self.a_wound_seen_by_the_fire(agent_index, bled_heavily);
        let state = &mut self.population.agents[patient].state;
        state.bleeding = 0.0;
        state.wound_dressed_until = turn_now + Self::A_DRESSING_IS_GOOD_FOR;
        if poulticed {
            state.wound_poulticed = true;
            self.world.poultices_laid += 1;
        }

        ActionResult::success()
            .with_drive_change(if who.is_some() { DriveType::Protection } else { DriveType::Rest }, -0.3)
            .with_energy_cost(1.0)
            .with_message("Bound up the wound".to_string())
    }

    /// `Action::CleanWound`: the dirt washed out of an open wound (#399),
    /// with water carried or water at hand. Washed, it turns half as often.
    pub(in crate::analytics) fn cleaning_a_wound(&mut self, agent_index: usize, who: Option<uuid::Uuid>) -> ActionResult {
        if !self.population.agents[agent_index].knows_how_to_do("CleanWound") {
            return ActionResult::failure("Nobody has thought to wash a wound".to_string());
        }
        let patient = match self.the_patient(agent_index, who) {
            Ok(patient) => patient,
            Err(refused) => return refused,
        };
        let state = &self.population.agents[patient].state;
        if state.an_open_wound <= 0.0 || state.wound_cleaned {
            return ActionResult::failure("There is no dirty wound to wash".to_string());
        }
        let here = self.population.agents[agent_index].state.position;
        let at_hand = self.water_to_wash_with_at(here);
        if !at_hand {
            let carer = &mut self.population.agents[agent_index];
            if carer.how_much_water_i_carry() < Self::WHAT_WASHING_A_WOUND_TAKES {
                return ActionResult::failure("No water to wash it with".to_string());
            }
            carer.draw_from_what_i_carry(Self::WHAT_WASHING_A_WOUND_TAKES);
        }
        self.population.agents[patient].state.wound_cleaned = true;
        self.world.wounds_cleaned += 1;

        ActionResult::success()
            .with_drive_change(if who.is_some() { DriveType::Protection } else { DriveType::Rest }, -0.2)
            .with_energy_cost(1.0)
            .with_message("Washed the wound".to_string())
    }

    /// Whether there is water, fresh or salt, a step from here to wash a
    /// wound in (#399).
    pub(in crate::analytics) fn water_to_wash_with_at(&self, here: (i32, i32, i32)) -> bool {
        self.world
            .nodes_near(crate::world::Position::new(here.0, here.1), Self::WITHIN_A_HANDS_REACH as u32)
            .any(|node| node.resource_type == crate::world::ResourceType::Water && node.what_can_be_taken() > 0)
    }

    /// How much carried water washing a wound takes (#399).
    pub(in crate::analytics) const WHAT_WASHING_A_WOUND_TAKES: f32 = 0.5;

    /// `Action::Cauterise`: a brand out of the fire laid on a wound that is
    /// bleeding (#400). It stops at once, the wound is seared half shut and
    /// clean, and it costs a burn and a fright.
    pub(in crate::analytics) fn cauterising(&mut self, agent_index: usize, who: Option<uuid::Uuid>) -> ActionResult {
        if !self.population.agents[agent_index].knows_how_to_do("Cauterise") {
            return ActionResult::failure("Nobody has thought to sear a wound".to_string());
        }
        let patient = match self.the_patient(agent_index, who) {
            Ok(patient) => patient,
            Err(refused) => return refused,
        };
        if self.population.agents[patient].state.bleeding < crate::agents::Agent::BLEEDING_WORTH_PRESSING {
            return ActionResult::failure("It is not bleeding enough to sear".to_string());
        }
        let here = self.population.agents[agent_index].state.position;
        if self.nearest_fire_from(here, Self::FIRE_REACH, true).is_none() {
            return ActionResult::failure("No fire to heat a brand in".to_string());
        }

        let patient_agent = &mut self.population.agents[patient];
        let state = &mut patient_agent.state;
        state.bleeding = 0.0;
        state.an_open_wound *= Self::WHAT_SEARING_LEAVES_OPEN;
        state.wound_cleaned = true;
        state.lose_health(Self::WHAT_A_BRAND_BURNS, crate::agents::AgentState::A_WOUND);
        let traits = patient_agent.traits.clone();
        patient_agent.emotions.add_fear_with_traits(
            crate::agents::EmotionSource::Event("the brand".to_string()),
            Self::WHAT_THE_BRAND_IS_IN_FRIGHT,
            &traits,
        );
        self.world.wounds_cauterised += 1;

        ActionResult::success()
            .with_drive_change(if who.is_some() { DriveType::Protection } else { DriveType::Rest }, -0.3)
            .with_energy_cost(1.0)
            .with_message("Seared the wound shut".to_string())
    }

    /// What of a wound a brand leaves open, what it burns, and what it is in
    /// fright (#400).
    pub(in crate::analytics) const WHAT_SEARING_LEAVES_OPEN: f32 = 0.5;
    pub(in crate::analytics) const WHAT_A_BRAND_BURNS: f32 = 4.0;
    pub(in crate::analytics) const WHAT_THE_BRAND_IS_IN_FRIGHT: f32 = 0.4;

    /// Bleeding heavy enough that somebody by a fire wonders about the fire
    /// (#400): half again what is worth pressing on.
    pub(in crate::analytics) const BLEEDING_HEAVY_ENOUGH_TO_SEAR: f32 = 2.0;

    /// `Action::StitchWound`: an open wound drawn shut with a point and a
    /// thread (#401). It bleeds no more and closes twice as fast.
    pub(in crate::analytics) fn stitching_a_wound(&mut self, agent_index: usize, who: Option<uuid::Uuid>) -> ActionResult {
        if !self.population.agents[agent_index].knows_how_to_do("StitchWound") {
            return ActionResult::failure("Nobody has thought to sew a wound".to_string());
        }
        let patient = match self.the_patient(agent_index, who) {
            Ok(patient) => patient,
            Err(refused) => return refused,
        };
        let state = &self.population.agents[patient].state;
        if state.an_open_wound < Self::A_WOUND_WORTH_STITCHING || state.wound_stitched {
            return ActionResult::failure("Nothing gaping to sew".to_string());
        }
        if !Self::has_something_to_pierce_with(&self.population.agents[agent_index]) {
            return ActionResult::failure("Nothing to make the holes with".to_string());
        }
        let carer = &mut self.population.agents[agent_index];
        let holding = |what: &str| carer.how_many_i_have(what);
        let Ok(using) = crate::environment::tags::what_would_be_used(Self::WHAT_A_DRESSING_TAKES, &holding) else {
            return ActionResult::failure("No thread to sew it with".to_string());
        };
        for (what, how_many) in using {
            carer.inventory.remove_item(what, how_many);
        }
        let state = &mut self.population.agents[patient].state;
        state.wound_stitched = true;
        state.bleeding = 0.0;
        self.world.wounds_stitched += 1;

        ActionResult::success()
            .with_drive_change(if who.is_some() { DriveType::Protection } else { DriveType::Rest }, -0.3)
            .with_energy_cost(2.0)
            .with_message("Sewed the wound shut".to_string())
    }

    /// How open a wound has to be to be worth sewing (#401).
    pub(in crate::analytics) const A_WOUND_WORTH_STITCHING: f32 = 0.4;

    /// `Action::SetBone`: a broken limb pulled straight and splinted with
    /// wood and a binding (#402). Set, it knits straight nine times in ten.
    pub(in crate::analytics) fn setting_a_bone(&mut self, agent_index: usize, who: uuid::Uuid) -> ActionResult {
        if !self.population.agents[agent_index].knows_how_to_do("SetBone") {
            return ActionResult::failure("Nobody has thought to set a bone".to_string());
        }
        let patient = match self.the_patient(agent_index, Some(who)) {
            Ok(patient) => patient,
            Err(refused) => return refused,
        };
        if patient == agent_index {
            return ActionResult::failure("Nobody can set their own".to_string());
        }
        let state = &self.population.agents[patient].state;
        if state.a_broken_limb <= 0.0 || state.limb_set {
            return ActionResult::failure("No break to set".to_string());
        }
        let carer = &mut self.population.agents[agent_index];
        let holding = |what: &str| carer.how_many_i_have(what);
        let Ok(binding) = crate::environment::tags::what_would_be_used(Self::WHAT_A_DRESSING_TAKES, &holding) else {
            return ActionResult::failure("Nothing to bind a splint with".to_string());
        };
        if carer.how_many_i_have("wood") < Self::WHAT_A_SPLINT_TAKES {
            return ActionResult::failure("No wood for a splint".to_string());
        }
        for (what, how_many) in binding {
            carer.inventory.remove_item(what, how_many);
        }
        carer.inventory.remove_item("wood", Self::WHAT_A_SPLINT_TAKES);

        let patient_agent = &mut self.population.agents[patient];
        patient_agent.state.limb_set = true;
        let traits = patient_agent.traits.clone();
        patient_agent.emotions.add_fear_with_traits(
            crate::agents::EmotionSource::Event("the bone pulled straight".to_string()),
            Self::WHAT_SETTING_IS_IN_FRIGHT,
            &traits,
        );
        self.world.bones_set += 1;

        ActionResult::success()
            .with_drive_change(DriveType::Protection, -0.3)
            .with_energy_cost(3.0)
            .with_message("Set the bone".to_string())
    }

    /// What a splint takes in wood, and what having a bone pulled straight is
    /// in fright (#402).
    pub(in crate::analytics) const WHAT_A_SPLINT_TAKES: u32 = 1;
    pub(in crate::analytics) const WHAT_SETTING_IS_IN_FRIGHT: f32 = 0.3;

    /// `Action::AttendBirth`: staying by somebody whose child is coming
    /// (#403). What the help is worth is reckoned at the birth.
    pub(in crate::analytics) fn attending_a_birth(&mut self, agent_index: usize, who: uuid::Uuid) -> ActionResult {
        let mother = match self.the_patient(agent_index, Some(who)) {
            Ok(mother) => mother,
            Err(refused) => return refused,
        };
        if self.population.agents[mother].pregnancy.is_none() {
            return ActionResult::failure("No child coming".to_string());
        }
        ActionResult::success()
            .with_drive_change(DriveType::Protection, -0.1)
            .with_energy_cost(0.5)
            .with_message("Stayed with the mother".to_string())
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

    /// A wound bleeding hard, seen to by somebody standing at a fire (#400):
    /// enough of them and the fire comes to mind.
    fn a_wound_seen_by_the_fire(&mut self, agent_index: usize, bleeding: f32) {
        if bleeding < Self::BLEEDING_HEAVY_ENOUGH_TO_SEAR {
            return;
        }
        let here = self.population.agents[agent_index].state.position;
        if self.nearest_fire_from(here, Self::FIRE_REACH, true).is_none() {
            return;
        }
        let carer = &mut self.population.agents[agent_index];
        carer.skills.made_one(Self::A_WOUND_BLED_BY_THE_FIRE);
        if carer.skills.times_made(Self::A_WOUND_BLED_BY_THE_FIRE) >= Self::WOUNDS_BLED_BY_THE_FIRE_BEFORE_SEARING {
            carer.it_comes_to_me("do:Cauterise", Self::WOUNDS_BLED_BY_THE_FIRE_BEFORE_SEARING);
        }
    }

    /// What the practice record calls a wound seen bleeding hard by a fire,
    /// and how many before the brand comes to mind (#400).
    pub(in crate::analytics) const A_WOUND_BLED_BY_THE_FIRE: &'static str = "woundbledbythefire";
    pub(in crate::analytics) const WOUNDS_BLED_BY_THE_FIRE_BEFORE_SEARING: u32 = 3;

    /// What it calls a wound bound with a point in the pack, and how many
    /// before sewing one comes to mind (#401).
    pub(in crate::analytics) const A_WOUND_BOUND_WITH_A_POINT_TO_HAND: &'static str = "woundboundwithapoint";
    pub(in crate::analytics) const WOUNDS_BOUND_BEFORE_SEWING_ONE: u32 = 3;

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
