// src/analytics/wanting/wounds.rs
//! What a wound asks of its owner and of the people round them, and water
//! taken along on a walk (#351).

use super::super::Simulation;
use crate::environment::Action;

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
        let now = self.current_turn;
        let can_bind = agent.knows_how_to_do("Bandage") && {
            let holding = |what: &str| agent.how_many_i_have(what);
            crate::environment::tags::what_would_be_used(Self::WHAT_A_DRESSING_TAKES, &holding).is_ok()
        };
        let wants_binding =
            |other: &crate::agents::Agent| other.state.an_open_wound > 0.0 && now >= other.state.wound_dressed_until;

        // One's own, first
        let mine = &agent.state;
        let instead = if mine.bleeding >= crate::agents::Agent::BLEEDING_WORTH_PRESSING {
            Some((if can_bind { Action::Bandage { who: None } } else { Action::PressOnWound { who: None } }, "wound: pressed on my own"))
        } else if can_bind && wants_binding(agent) {
            Some((Action::Bandage { who: None }, "wound: bound up my own"))
        } else {
            self.somebody_elses_wound(agent_index, can_bind)
        };

        match instead {
            Some((instead, why)) => {
                *self.what_a_threat_came_to.entry(why.to_string()).or_insert(0) += 1;
                instead
            }
            None => action,
        }
    }

    /// Somebody else's wound that this one should see to, and how.
    fn somebody_elses_wound(&self, agent_index: usize, can_bind: bool) -> Option<(Action, &'static str)> {
        let agent = &self.population.agents[agent_index];
        let here = agent.state.position;
        let now = self.current_turn;
        let apart = |there: (i32, i32, i32)| (here.0 - there.0).abs().max((here.1 - there.1).abs());
        let is_mine = |other: &crate::agents::Agent| {
            other.parent_ids.contains(&agent.id)
                || agent.parent_ids.contains(&other.id)
                || other.parent_ids.iter().any(|p| agent.parent_ids.contains(p))
                || agent.my_partner() == Some(other.id)
        };

        let mut best: Option<(i32, &crate::agents::Agent)> = None;
        for other in self.population.agents.iter() {
            if other.id == agent.id || !other.state.is_alive {
                continue;
            }
            let bleeding = other.state.bleeding >= crate::agents::Agent::BLEEDING_WORTH_PRESSING;
            let to_bind = can_bind && other.state.an_open_wound > 0.0 && now >= other.state.wound_dressed_until;
            if !bleeding && !to_bind {
                continue;
            }
            let paces = apart(other.state.position);
            let near_enough = paces <= Self::WITHIN_A_HANDS_REACH
                || (is_mine(other) && paces <= Self::HOW_FAR_YOU_WILL_GO_TO_A_WOUND);
            if near_enough && best.map(|(so_far, _)| paces < so_far).unwrap_or(true) {
                best = Some((paces, other));
            }
        }

        let (paces, other) = best?;
        if paces > Self::WITHIN_A_HANDS_REACH {
            return Some((Action::Move { target: other.state.position }, "wound: went to one of mine"));
        }
        if can_bind {
            return Some((Action::Bandage { who: Some(other.id) }, "wound: bound up somebody's"));
        }
        Some((Action::PressOnWound { who: Some(other.id) }, "wound: pressed on somebody's"))
    }

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
                if let Some((verb, to)) = agent.what_vessel_i_would_rather_have() {
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
