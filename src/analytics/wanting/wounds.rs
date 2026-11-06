// src/analytics/wanting/wounds.rs
//! What a wound asks of its owner and of the people round them, and water
//! taken along on a walk (#351).

use super::super::Simulation;
use crate::environment::Action;

/// What somebody knows how to do for a wound and has the makings of, now.
struct WhatTheseHandsCanDo {
    bind: bool,
    wash: bool,
    sear: bool,
    sew: bool,
    set: bool,
}

impl Simulation {
    /// What a bleeding wound - one's own, or somebody's near - asks for
    /// first.
    ///
    /// Bleeding does not wait on a drive coming round: a person losing blood
    /// presses on it, or binds it if they know how and have something to bind
    /// it with, before anything but running for their life. Somebody near a
    /// person who is bleeding does the same for them; one of their own
    /// bleeding across the camp is walked to. And an open wound that has
    /// stopped bleeding is bound up by whoever knows how, while the strip is
    /// in the pack.
    pub(in crate::analytics) fn what_a_wound_asks(
        &mut self,
        agent_index: usize,
        action: Action,
        running_away: bool,
    ) -> Action {
        if running_away {
            return action;
        }
        let agent = &self.population.agents[agent_index];
        if agent.state.life_stage == crate::agents::LifeStage::Infant {
            return action;
        }
        let can = self.what_these_hands_can_do_for_a_wound(agent_index);

        // One's own, first
        let instead = self
            .what_my_own_wound_asks(agent_index, &can)
            .or_else(|| self.somebody_elses_wound(agent_index, &can))
            .or_else(|| self.what_a_birth_asks(agent_index));

        match instead {
            Some((instead, why)) => {
                *self.what_a_threat_came_to.entry(why.to_string()).or_insert(0) += 1;
                instead
            }
            None => action,
        }
    }

    /// What this one knows how to do for a wound and has the makings of, now
    /// (#351, #399-#402).
    fn what_these_hands_can_do_for_a_wound(&self, agent_index: usize) -> WhatTheseHandsCanDo {
        let agent = &self.population.agents[agent_index];
        let here = agent.state.position;
        let holding = |what: &str| agent.how_many_i_have(what);
        let a_strip = crate::environment::tags::what_would_be_used(Self::WHAT_A_DRESSING_TAKES, &holding).is_ok();
        WhatTheseHandsCanDo {
            bind: agent.knows_how_to_do("Bandage") && a_strip,
            wash: agent.knows_how_to_do("CleanWound")
                && (agent.how_much_water_i_carry() >= Self::WHAT_WASHING_A_WOUND_TAKES || self.water_to_wash_with_at(here)),
            sear: agent.knows_how_to_do("Cauterise") && self.nearest_fire_from(here, Self::FIRE_REACH, true).is_some(),
            sew: agent.knows_how_to_do("StitchWound") && a_strip && Self::has_something_to_pierce_with(agent),
            set: agent.knows_how_to_do("SetBone") && a_strip && agent.how_many_i_have("wood") >= Self::WHAT_A_SPLINT_TAKES,
        }
    }

    /// What one's own wound asks for (#351, #399-#401).
    ///
    /// Bleeding hard, somebody by a fire who knows the brand sears it, unless
    /// they are too frightened of it; otherwise it is bound, or pressed on.
    /// Once it has stopped, it is washed, sewn and bound, by whoever knows how.
    fn what_my_own_wound_asks(&self, agent_index: usize, can: &WhatTheseHandsCanDo) -> Option<(Action, &'static str)> {
        use crate::core::traits::Trait;

        let agent = &self.population.agents[agent_index];
        let mine = &agent.state;
        if mine.bleeding >= crate::agents::Agent::BLEEDING_WORTH_PRESSING {
            let dreads_the_brand = agent.traits.has(Trait::Coward) || agent.traits.has(Trait::Anxious);
            if can.sear && mine.bleeding >= Self::BLEEDING_HEAVY_ENOUGH_TO_SEAR && !dreads_the_brand {
                return Some((Action::Cauterise { who: None }, "wound: seared my own"));
            }
            if can.bind {
                return Some((Action::Bandage { who: None }, "wound: pressed on my own"));
            }
            return Some((Action::PressOnWound { who: None }, "wound: pressed on my own"));
        }
        self.what_an_open_wound_wants(agent, can, None, "my own")
    }

    /// What a wound that has stopped bleeding wants next, of somebody who can
    /// do it (#399, #401, #351): washing, sewing, binding.
    fn what_an_open_wound_wants(
        &self,
        patient: &crate::agents::Agent,
        can: &WhatTheseHandsCanDo,
        who: Option<uuid::Uuid>,
        whose: &'static str,
    ) -> Option<(Action, &'static str)> {
        let state = &patient.state;
        if state.an_open_wound <= 0.0 {
            return None;
        }
        let mine = whose == "my own";
        if can.wash && !state.wound_cleaned {
            return Some((Action::CleanWound { who }, if mine { "wound: washed my own" } else { "wound: washed somebody's" }));
        }
        if can.sew && !state.wound_stitched && state.an_open_wound >= Self::A_WOUND_WORTH_STITCHING {
            return Some((Action::StitchWound { who }, if mine { "wound: sewed my own" } else { "wound: sewed somebody's" }));
        }
        if can.bind && self.current_turn >= state.wound_dressed_until {
            return Some((Action::Bandage { who }, if mine { "wound: bound up my own" } else { "wound: bound up somebody's" }));
        }
        None
    }

    /// How far this one would go to see to somebody else (#351, #399-#403):
    /// across the camp for their own, or for anybody if they are kind; a hand's
    /// reach for anybody else; and not at all, for anybody not their own, if
    /// they are cold or cruel.
    pub(in crate::analytics) fn how_far_i_would_go_for(
        agent: &crate::agents::Agent,
        other: &crate::agents::Agent,
    ) -> i32 {
        use crate::core::traits::Trait;

        let is_mine = other.parent_ids.contains(&agent.id)
            || agent.parent_ids.contains(&other.id)
            || other.parent_ids.iter().any(|p| agent.parent_ids.contains(p))
            || agent.my_partner() == Some(other.id);
        if is_mine || agent.traits.has(Trait::KindHearted) || agent.traits.has(Trait::Altruist) {
            return Self::HOW_FAR_YOU_WILL_GO_TO_A_WOUND;
        }
        if agent.traits.has(Trait::ColdHearted) || agent.traits.has(Trait::Uncaring) || agent.traits.has(Trait::Cruel) {
            return -1;
        }
        Self::WITHIN_A_HANDS_REACH
    }

    /// Somebody else's wound or break that this one should see to, and how.
    fn somebody_elses_wound(&self, agent_index: usize, can: &WhatTheseHandsCanDo) -> Option<(Action, &'static str)> {
        use crate::core::traits::Trait;

        let agent = &self.population.agents[agent_index];
        let here = agent.state.position;
        let apart = |there: (i32, i32, i32)| (here.0 - there.0).abs().max((here.1 - there.1).abs());

        let mut best: Option<(i32, &crate::agents::Agent, (Action, &'static str))> = None;
        for other in self.population.agents.iter() {
            if other.id == agent.id || !other.state.is_alive {
                continue;
            }
            let paces = apart(other.state.position);
            if paces > Self::how_far_i_would_go_for(agent, other) || best.as_ref().is_some_and(|(so_far, ..)| paces >= *so_far) {
                continue;
            }
            let who = Some(other.id);
            let state = &other.state;
            let wanted = if state.bleeding >= crate::agents::Agent::BLEEDING_WORTH_PRESSING {
                // A kind heart cannot bring itself to lay a brand on somebody
                if can.sear && state.bleeding >= Self::BLEEDING_HEAVY_ENOUGH_TO_SEAR && !agent.traits.has(Trait::KindHearted) {
                    Some((Action::Cauterise { who }, "wound: seared somebody's"))
                } else if can.bind {
                    Some((Action::Bandage { who }, "wound: bound up somebody's"))
                } else {
                    Some((Action::PressOnWound { who }, "wound: pressed on somebody's"))
                }
            } else if can.set && state.a_broken_limb > 0.0 && !state.limb_set {
                Some((Action::SetBone { who: other.id }, "bone: set somebody's"))
            } else {
                self.what_an_open_wound_wants(other, can, who, "somebody's")
            };
            if let Some(wanted) = wanted {
                best = Some((paces, other, wanted));
            }
        }

        let (paces, other, wanted) = best?;
        if paces > Self::WITHIN_A_HANDS_REACH {
            return Some((Action::Move { target: other.state.position }, "wound: went to one of mine"));
        }
        Some(wanted)
    }

    /// Somebody whose child is coming, that this one knows how to help and
    /// would go to (#403): walked to, and stayed with until it comes.
    fn what_a_birth_asks(&self, agent_index: usize) -> Option<(Action, &'static str)> {
        let agent = &self.population.agents[agent_index];
        if !agent.knows_how_to_do("AttendBirth") || agent.state.years_old() < crate::agents::Agent::OLD_ENOUGH_TO_KEEP_A_KIT {
            return None;
        }
        let here = agent.state.position;
        let now = self.current_turn;
        let apart = |there: (i32, i32, i32)| (here.0 - there.0).abs().max((here.1 - there.1).abs());
        let (paces, mother) = self
            .population
            .agents
            .iter()
            .filter(|other| other.id != agent.id && other.state.is_alive)
            .filter(|other| {
                other
                    .pregnancy
                    .as_ref()
                    .is_some_and(|carrying| carrying.due_turn <= now + Self::A_BIRTH_IS_NEAR)
            })
            .map(|other| (apart(other.state.position), other))
            .filter(|(paces, other)| *paces <= Self::how_far_i_would_go_for(agent, other).max(Self::AT_A_BIRTH))
            .min_by_key(|(paces, _)| *paces)?;
        if paces > Self::AT_A_BIRTH - 1 {
            return Some((Action::Move { target: mother.state.position }, "birth: went to the mother"));
        }
        Some((Action::AttendBirth { who: mother.id }, "birth: stayed with the mother"))
    }

    /// How near the birth has to be for a midwife to go and stay (#403): a
    /// day.
    pub(in crate::analytics) const A_BIRTH_IS_NEAR: u32 = crate::environment::seasons::TICKS_PER_DAY;

    /// How far somebody goes to one of their own who is bleeding: across the
    /// camp.
    pub(in crate::analytics) const HOW_FAR_YOU_WILL_GO_TO_A_WOUND: i32 = 12;

    /// Water taken along: somebody setting off on a walk past the water,
    /// with room in a vessel, fills it first (#351); and with no vessel,
    /// makes one first if they can (#352).
    ///
    /// A vessel was only ever filled as a side effect of drinking, and
    /// `top_up_before_you_need_it` is the last thing tried before waiting. The
    /// filling here is the drink at the source, which fills whatever is
    /// carried.
    pub(in crate::analytics) fn water_for_the_walk(&self, agent_index: usize, action: Action, running_away: bool) -> Action {
        if running_away {
            return action;
        }
        let Action::Move { target } = action else {
            return action;
        };
        let agent = &self.population.agents[agent_index];
        let here = agent.state.position;
        let going = (target.0 - here.0).abs().max((target.1 - here.1).abs());
        if going <= Self::FORAGE_RADIUS as i32 {
            return action;
        }
        if !self.drinkable_water_within_reach(agent, here) {
            return action;
        }

        // Nothing to carry it in, and the makings of something and a blade
        // to make it with: make it now, beside the water, and fill it next
        // turn (#352). A vessel was made only when the Utility drive won a
        // turn, which it rarely does - 1.2% of agent-days had one.
        if agent.what_i_can_hold_water_in() == 0 {
            if agent.what_i_have_to_work_with(crate::agents::SkillType::Crafting).is_some() {
                let a_fire_to_hand = self.nearest_fire_from(here, Self::FIRE_REACH, true).is_some();
                if let Some((verb, to)) = agent.what_vessel_i_would_rather_have_with(a_fire_to_hand) {
                    return Action::Work { verb: verb.to_string(), to: to.to_string() };
                }
            }
            return action;
        }

        let room: f32 = agent
            .inventory
            .get_all_items()
            .values()
            .filter(|item| item.is_container())
            .map(|item| item.max_capacity.unwrap_or(0.0) - item.fill_level.unwrap_or(0.0))
            .sum();
        if room < 1.0 {
            return action;
        }
        Action::Gather { resource_type: "water".to_string() }
    }
}
