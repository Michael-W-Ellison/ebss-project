// src/analytics/happening/bodies.rs
//! What a body that came to harm shows the people round it (#399, #402), and
//! how a birth goes (#403).

use super::super::Simulation;
use crate::agents::{Agent, AgentState, Population};

impl Simulation {
    /// Hand on what happened to anybody's body this turn to the people who
    /// would have seen it, and count it (#399, #402).
    ///
    /// A wound that turned, seen twice, makes somebody apt to wash the next
    /// one; a bone seen to knit crooked, twice, makes somebody apt to pull
    /// the next one straight. Both at the pace, like any idea.
    pub(in crate::analytics) fn what_the_camp_saw_of_bodies(&mut self) {
        let mut seen: Vec<((i32, i32, i32), String)> = Vec::new();
        for agent in self.population.agents.iter_mut() {
            let at = agent.state.position;
            seen.extend(agent.state.to_be_seen.drain(..).map(|what| (at, what)));
        }

        for (at, what) in seen {
            let lesson = match what.as_str() {
                AgentState::A_WOUND_TURNED => {
                    self.world.wounds_turned += 1;
                    Some((Self::A_WOUND_SEEN_TO_TURN, "do:CleanWound", Self::WOUNDS_SEEN_TO_TURN_BEFORE_WASHING))
                }
                AgentState::A_BONE_BROKE => {
                    self.world.bones_broken += 1;
                    None
                }
                AgentState::A_BONE_KNIT_STRAIGHT => {
                    self.world.bones_knit_straight += 1;
                    None
                }
                AgentState::A_BONE_KNIT_CROOKED => {
                    self.world.bones_knit_crooked += 1;
                    Some((Self::A_CROOKED_BONE_SEEN, "do:SetBone", Self::CROOKED_BONES_SEEN_BEFORE_SETTING))
                }
                _ => None,
            };
            let Some((record, idea, needed)) = lesson else {
                continue;
            };
            for agent in self.population.agents.iter_mut() {
                if !agent.state.is_alive
                    || agent.state.years_old() < Agent::OLD_ENOUGH_TO_KEEP_A_KIT
                    || Self::paces_between(agent.state.position, at) > Self::HOW_FAR_YOU_WILL_GO_TO_A_WOUND
                {
                    continue;
                }
                agent.skills.made_one(record);
                if agent.skills.times_made(record) >= needed {
                    agent.it_comes_to_me(idea, needed);
                }
            }
        }
    }

    /// How a birth goes (#403): the population's roll, see
    /// `Population::how_the_birth_went`, with what it took counted into the
    /// world.
    pub(in crate::analytics) fn how_the_birth_went(&mut self, mother_index: usize) -> bool {
        let lived = self.population.how_the_birth_went(mother_index);
        self.count_the_hard_births();
        lived
    }

    /// Take the population's count of births that went hard into the world's
    /// (#403).
    pub(in crate::analytics) fn count_the_hard_births(&mut self) {
        let tally = std::mem::take(&mut self.population.stats.births_gone_hard);
        self.world.hard_births += tally.hard;
        self.world.children_lost_at_birth += tally.children_lost;
        self.world.mothers_lost_in_childbirth += tally.mothers_lost;
        self.world.births_with_a_midwife += tally.with_a_midwife;
    }

    fn paces_between(one: (i32, i32, i32), other: (i32, i32, i32)) -> i32 {
        (one.0 - other.0).abs().max((one.1 - other.1).abs())
    }

    /// What the practice record calls a wound seen to turn, and how many
    /// before somebody washes the next (#399).
    pub(in crate::analytics) const A_WOUND_SEEN_TO_TURN: &'static str = "woundseentoturn";
    pub(in crate::analytics) const WOUNDS_SEEN_TO_TURN_BEFORE_WASHING: u32 = 2;

    /// What it calls a bone seen to knit crooked, and how many before
    /// somebody sets the next (#402).
    pub(in crate::analytics) const A_CROOKED_BONE_SEEN: &'static str = "crookedboneseen";
    pub(in crate::analytics) const CROOKED_BONES_SEEN_BEFORE_SETTING: u32 = 2;

    /// How near somebody has to be to be at a birth, and how near a birth has
    /// to be before a midwife goes to it (#403) - the population's figures.
    pub(in crate::analytics) const AT_A_BIRTH: i32 = Population::AT_A_BIRTH;
}
