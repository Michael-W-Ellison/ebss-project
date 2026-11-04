// src/analytics/wanting/roles.rs
//! Division of labour in food gathering (#407): the governance path's first
//! stage, "recognized roles by age, skill" and "division of labor in food
//! gathering".
//!
//! Everybody picked how to feed themselves by what they had been right
//! about before, and nobody looked at anybody else. Somebody who knows to
//! looks at the camp: the work where their hand is best against the others'
//! is the work they lean to, and they leave to the others what the others do
//! better. It is comparative advantage, and it is the hand, not the person:
//! a poor hunter in a camp of worse ones is the camp's hunter.

use super::super::Simulation;
use super::strategy::Strategy;
use crate::agents::{Agent, SkillType};
use crate::core::traits::Trait;

impl Simulation {
    /// How much one's standing at a way of getting food weighs on it, at
    /// most: a quarter more relief for the way one stands best at against
    /// the camp, and a quarter less for the way one stands worst at.
    pub(in crate::analytics) const WHAT_A_ROLE_WEIGHS: f32 = 0.5;
    pub(in crate::analytics) const THE_MOST_A_ROLE_TIPS_IT: f32 = 0.25;

    /// The ways of getting food that ask for a hand at something, and what.
    pub(in crate::analytics) const THE_WAYS_A_HAND_DECIDES: [(Strategy, SkillType); 3] = [
        (Strategy::GatherWildFood, SkillType::Herbalism),
        (Strategy::FishLocalWaters, SkillType::Fishing),
        (Strategy::HuntLocalAnimals, SkillType::Hunting),
    ];

    /// What the practice record calls a campmate seen bringing in food at a
    /// work they have a better hand at, and how many before somebody thinks
    /// to leave it to them.
    pub(in crate::analytics) const A_BETTER_HAND_SEEN: &'static str = "betterhandseen";
    pub(in crate::analytics) const BETTER_HANDS_SEEN_BEFORE_A_ROLE: u32 = 5;

    /// How much better a hand has to be to be seen to be: about four levels
    /// of skill, a third again of an ordinary hand.
    pub(in crate::analytics) const A_HAND_PLAINLY_BETTER_BY: f32 = 0.3;

    /// How near somebody has to be to see what came in: a camp's width.
    pub(in crate::analytics) const NEAR_ENOUGH_TO_SEE_WHAT_CAME_IN: i32 = 12;

    /// How lately somebody has to have been at the same work to set what
    /// they brought in beside what somebody else did: half a day (#407).
    pub(in crate::analytics) const AT_THE_SAME_WORK_WITHIN: u32 = crate::environment::seasons::TICKS_PER_DAY / 2;

    /// Whether this one takes the work the camp's hands leave them, knowing
    /// to: not the stubborn, the rebel or the proud, who do what they would
    /// have done whoever else is better at it.
    pub(in crate::analytics) fn would_take_a_role(agent: &Agent) -> bool {
        agent.knows_the_way("division_of_labour")
            && !agent.traits.has(Trait::Stubborn)
            && !agent.traits.has(Trait::Rebel)
            && !agent.traits.has(Trait::Proud)
    }

    /// What a way of getting food is worth to this one for their standing at
    /// it in their camp, as a multiple of its relief: one for anybody who
    /// does not divide the work, for a way no hand decides, and for somebody
    /// with nobody to divide it with.
    pub(in crate::analytics) fn what_my_hand_is_worth_to_the_camp(&self, agent: &Agent, way: Strategy) -> f32 {
        let Some(&(_, skill)) = Self::THE_WAYS_A_HAND_DECIDES.iter().find(|(w, _)| *w == way) else {
            return 1.0;
        };
        if !Self::would_take_a_role(agent) {
            return 1.0;
        }
        let mates = self.who_lives_near(Self::where_this_one_lives(agent));
        if mates.iter().filter(|&&index| self.population.agents[index].id != agent.id).count() == 0 {
            return 1.0;
        }
        // Where I stand at each, against the camp's hands at it
        let standing = |at: SkillType| {
            let camps: f32 = mates.iter().map(|&index| self.population.agents[index].skills.hand_for(at)).sum::<f32>()
                / mates.len() as f32;
            agent.skills.hand_for(at) / camps.max(f32::EPSILON)
        };
        let mine: Vec<f32> = Self::THE_WAYS_A_HAND_DECIDES.iter().map(|(_, at)| standing(*at)).collect();
        let on_the_whole = mine.iter().sum::<f32>() / mine.len() as f32;
        let here = standing(skill) / on_the_whole.max(f32::EPSILON);
        self.ways_weighed_by_the_hand.set(self.ways_weighed_by_the_hand.get() + 1);
        (1.0 + Self::WHAT_A_ROLE_WEIGHS * (here - 1.0))
            .clamp(1.0 - Self::THE_MOST_A_ROLE_TIPS_IT, 1.0 + Self::THE_MOST_A_ROLE_TIPS_IT)
    }

    /// Somebody has brought food in at a work: whoever grown is near enough
    /// to see it, has been at the same work themselves within half a day, and
    /// has a plainly worse hand at it, has seen a better hand: they know what
    /// they brought in, and can set it beside this.
    /// Five times, and they are apt to think of leaving it to the better ones
    /// (`know:division_of_labour`, at the pace).
    pub(in crate::analytics) fn a_hand_seen_at_work(&mut self, worker: usize, at: SkillType) {
        let here = self.population.agents[worker].state.position;
        let theirs = self.population.agents[worker].skills.hand_for(at);
        let now = self.current_turn;
        let mut seen = 0;
        for (index, agent) in self.population.agents.iter_mut().enumerate() {
            if index == worker
                || !agent.state.is_alive
                || agent.state.years_old() < Agent::OLD_ENOUGH_TO_KEEP_A_KIT
                || (agent.state.position.0 - here.0).abs().max((agent.state.position.1 - here.1).abs())
                    > Self::NEAR_ENOUGH_TO_SEE_WHAT_CAME_IN
                || agent.skills.hand_for(at) + Self::A_HAND_PLAINLY_BETTER_BY > theirs
                || !agent
                    .skills
                    .get_skill_if_exists(at)
                    .is_some_and(|mine| now.saturating_sub(mine.last_used) <= Self::AT_THE_SAME_WORK_WITHIN)
            {
                continue;
            }
            seen += 1;
            agent.skills.made_one(Self::A_BETTER_HAND_SEEN);
            if agent.skills.times_made(Self::A_BETTER_HAND_SEEN) >= Self::BETTER_HANDS_SEEN_BEFORE_A_ROLE {
                agent.it_comes_to_me("know:division_of_labour", Self::BETTER_HANDS_SEEN_BEFORE_A_ROLE);
            }
        }
        self.world.better_hands_seen += seen;
    }
}
