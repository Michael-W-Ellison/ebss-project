// src/analytics/happening/bodies.rs
//! What a body that came to harm shows the people round it (#399, #402), and
//! how a birth goes (#403).

use super::super::Simulation;
use crate::agents::{Agent, AgentState};

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

    /// How a birth goes (#403), rolled when the child comes. Returns whether
    /// the child lived.
    ///
    /// Most births go as they should. One in eight goes hard: the child is
    /// lost two times in five, the mother one time in ten, and a mother who
    /// lives through it is the worse for it. Somebody at hand who knows how to
    /// help a birth halves both.
    ///
    /// Everybody grown who is at a birth has seen one; three, and somebody is
    /// apt to know how to help the next (`do:AttendBirth`).
    pub(in crate::analytics) fn how_the_birth_went(&mut self, mother_index: usize) -> bool {
        use rand::Rng;

        let mother_at = self.population.agents[mother_index].state.position;
        let mut helped = false;
        for (index, agent) in self.population.agents.iter_mut().enumerate() {
            if !agent.state.is_alive
                || agent.state.years_old() < Agent::OLD_ENOUGH_TO_KEEP_A_KIT
                || Self::paces_between(agent.state.position, mother_at) > Self::AT_A_BIRTH
            {
                continue;
            }
            if index != mother_index && agent.knows_how_to_do("AttendBirth") {
                helped = true;
            }
            agent.skills.made_one(Self::A_BIRTH_SEEN);
            if agent.skills.times_made(Self::A_BIRTH_SEEN) >= Self::BIRTHS_SEEN_BEFORE_HELPING {
                agent.it_comes_to_me("do:AttendBirth", Self::BIRTHS_SEEN_BEFORE_HELPING);
            }
        }

        let mut dice = crate::core::dice::roll();
        if !dice.gen_bool(Self::A_BIRTH_GOES_HARD) {
            return true;
        }
        self.world.hard_births += 1;
        if helped {
            self.world.births_with_a_midwife += 1;
        }
        let help = if helped { Self::WHAT_A_MIDWIFE_LEAVES } else { 1.0 };
        let child_lost = dice.gen_bool(Self::A_HARD_BIRTH_LOSES_THE_CHILD * help);
        let mother_lost = dice.gen_bool(Self::A_HARD_BIRTH_TAKES_THE_MOTHER * help);
        drop(dice);

        let mother = &mut self.population.agents[mother_index];
        if mother_lost {
            let all_of_it = mother.state.health;
            mother.state.lose_health(all_of_it, AgentState::CHILDBIRTH);
            self.world.mothers_lost_in_childbirth += 1;
        } else {
            mother.state.lose_health(Self::WHAT_A_HARD_BIRTH_TAKES, AgentState::CHILDBIRTH);
        }
        if child_lost {
            self.world.children_lost_at_birth += 1;
            let traits = mother.traits.clone();
            mother.emotions.add_sadness_with_traits(
                crate::agents::EmotionSource::Event("a child lost at birth".to_string()),
                Self::WHAT_A_CHILD_LOST_IS_IN_GRIEF,
                &traits,
            );
        }
        !child_lost
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

    /// What it calls a birth seen, and how many before somebody knows how to
    /// help one (#403).
    pub(in crate::analytics) const A_BIRTH_SEEN: &'static str = "birthseen";
    pub(in crate::analytics) const BIRTHS_SEEN_BEFORE_HELPING: u32 = 3;

    /// How near somebody has to be to be at a birth (#403).
    pub(in crate::analytics) const AT_A_BIRTH: i32 = 2;

    /// How often a birth goes hard, and what a hard one costs (#403): the
    /// child two times in five, the mother one in ten, and otherwise a fifth
    /// of her health. What a midwife leaves of those odds: half.
    pub(in crate::analytics) const A_BIRTH_GOES_HARD: f64 = 0.12;
    pub(in crate::analytics) const A_HARD_BIRTH_LOSES_THE_CHILD: f64 = 0.4;
    pub(in crate::analytics) const A_HARD_BIRTH_TAKES_THE_MOTHER: f64 = 0.1;
    pub(in crate::analytics) const WHAT_A_HARD_BIRTH_TAKES: f32 = 20.0;
    pub(in crate::analytics) const WHAT_A_MIDWIFE_LEAVES: f64 = 0.5;
    pub(in crate::analytics) const WHAT_A_CHILD_LOST_IS_IN_GRIEF: f32 = 0.8;
}
