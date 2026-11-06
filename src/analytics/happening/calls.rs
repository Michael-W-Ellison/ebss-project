// src/analytics/happening/calls.rs
//! A call raised at a beast (#406): "standardized calls for danger" in the
//! communication path's first stage.
//!
//! A beast that comes at somebody and stops short has been seen, by that
//! one, and nobody else is told: the next person to walk into its ground
//! finds out the way the first did, or worse. Somebody who knows the call
//! raises it, and everybody in earshot of it knows where the thing is.

use super::super::Simulation;
use crate::agents::Agent;
use crate::core::traits::Trait;

impl Simulation {
    /// How far a call carries, in paces: a shout across open ground, four
    /// times as far as talk (`Population::EARSHOT`).
    pub(in crate::analytics) const A_CALL_CARRIES: i32 = 25;

    /// How near to a beast a sleeper has to lie to be woken by a call about
    /// it: inside the ground it would charge, and a few paces more.
    pub(in crate::analytics) const NEAR_ENOUGH_TO_BE_WOKEN: i32 = Self::A_BEASTS_GROUND + 5;

    /// How long a beast seen stays news: a day.
    pub(in crate::analytics) const A_BEAST_SEEN_IS_NEWS_FOR: u32 = crate::environment::seasons::TICKS_PER_DAY;

    /// How near the place a beast was seen somebody has to be struck for a
    /// call to have reached them: within what a call carries.
    pub(in crate::analytics) const WHERE_A_CALL_WOULD_HAVE_REACHED: i32 = Self::A_CALL_CARRIES;

    /// What the practice record calls a blow from a beast somebody else had
    /// been warned off, and how many before somebody thinks to call out.
    pub(in crate::analytics) const A_BLOW_A_CALL_WOULD_HAVE_SPARED: &'static str = "blowacallwouldhavespared";
    pub(in crate::analytics) const BLOWS_BEFORE_A_CALL: u32 = 2;

    /// Whether this one raises the call, knowing it: anybody who knows it and
    /// has a voice, but not the cold-hearted, the uncaring or the callous, who
    /// have got themselves clear and see no reason to shout for anybody else.
    pub(in crate::analytics) fn would_raise_the_call(agent: &Agent) -> bool {
        agent.knows_the_way("calls")
            && !agent.traits.has(Trait::Mute)
            && !agent.traits.has(Trait::ColdHearted)
            && !agent.traits.has(Trait::Uncaring)
            && !agent.traits.has(Trait::Callous)
    }

    /// A beast has warned somebody off: note it, and if they would, they call
    /// it out (#406). Everybody awake within a call who can hear knows where
    /// it is, as though they had seen it; anybody asleep near it is woken.
    pub(in crate::analytics) fn a_beast_was_seen(
        &mut self,
        seen_by: usize,
        species_id: &str,
        called: &str,
        at: (i32, i32),
    ) {
        let now = self.current_turn;
        let at = (at.0, at.1, 0);
        let who = self.population.agents[seen_by].id;
        self.world
            .beasts_lately_seen
            .retain(|(_, _, when, _, _)| now.saturating_sub(*when) <= Self::A_BEAST_SEEN_IS_NEWS_FOR);
        let calls = Self::would_raise_the_call(&self.population.agents[seen_by]);
        self.world.beasts_lately_seen.push((at, species_id.to_string(), now, who, calls));
        if !calls {
            return;
        }
        let from = self.population.agents[seen_by].state.position;
        self.world.calls_raised += 1;
        let mut heard = 0;
        for (index, agent) in self.population.agents.iter_mut().enumerate() {
            if index == seen_by
                || !agent.state.is_alive
                || agent.traits.has(Trait::Deaf)
                || (agent.state.position.0 - from.0).abs().max((agent.state.position.1 - from.1).abs())
                    > Self::A_CALL_CARRIES
            {
                continue;
            }
            heard += 1;
            agent
                .exploration_knowledge
                .saw_danger(crate::world::Position::new(at.0, at.1), called, 1.0, now);
            let near_it = (agent.state.position.0 - at.0).abs().max((agent.state.position.1 - at.1).abs())
                <= Self::NEAR_ENOUGH_TO_BE_WOKEN;
            if near_it {
                agent.busy_until = 0;
                agent.state.next_decision_at = agent.state.next_decision_at.min(now);
            }
        }
        self.world.heard_a_call += heard;
    }

    /// A beast has struck somebody. If somebody else was warned off by its
    /// kind near here within the day, and called nothing, a call would have
    /// reached them: twice,
    /// and somebody is apt to think of calling out (`know:calls`, at the
    /// pace).
    pub(in crate::analytics) fn a_blow_from_a_beast_seen_before(&mut self, struck: usize, species_id: &str) {
        let now = self.current_turn;
        let agent = &self.population.agents[struck];
        let (me, here) = (agent.id, agent.state.position);
        let had_been_seen = self.world.beasts_lately_seen.iter().any(|(at, kind, when, by, called)| {
            *by != me
                && !called
                && kind == species_id
                && now.saturating_sub(*when) <= Self::A_BEAST_SEEN_IS_NEWS_FOR
                && (at.0 - here.0).abs().max((at.1 - here.1).abs()) <= Self::WHERE_A_CALL_WOULD_HAVE_REACHED
        });
        if !had_been_seen {
            return;
        }
        self.world.blows_a_call_would_have_spared += 1;
        let agent = &mut self.population.agents[struck];
        agent.skills.made_one(Self::A_BLOW_A_CALL_WOULD_HAVE_SPARED);
        if agent.skills.times_made(Self::A_BLOW_A_CALL_WOULD_HAVE_SPARED) >= Self::BLOWS_BEFORE_A_CALL {
            agent.it_comes_to_me("know:calls", Self::BLOWS_BEFORE_A_CALL);
        }
    }
}
