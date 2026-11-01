// src/analytics/happening/teaching.rs
//! The young are taught what their camp knows (ISSUES_FOUND #388).
//!
//! What somebody found out died with them, unless it was one of the few
//! things learned by watching it done (sowing, breaking ground, burning,
//! ditching, keeping beasts). Ways of working - selective harvesting,
//! coppicing - and every making found out - the hoe, the water skin,
//! leather - went into the ground with whoever had them. At the pace a people
//! finds things out (#389) that would lose nearly all of it.
//!
//! So a child or an adolescent is taught, a little at a time, what their
//! parents and the grown people of their camp know and they do not. What a
//! camp knows outlives the one who found it, and is lost when the camp is.
//!
//! And who they are bears on all of it (#390): how readily a child learns
//! and a grown person teaches, what a lesson does between them, and what a
//! camp makes of one of its own coming to something first.

use super::super::Simulation;
use crate::agents::LifeStage;

impl Simulation {
    /// The chance, a day, that a child learns one more thing the grown
    /// people about them know: a thing a month, so a childhood takes in a
    /// hundred or more.
    pub(in crate::analytics) const A_LESSON_A_DAY: f64 = 1.0 / 30.0;

    /// Once a day, each child and adolescent may learn one thing a parent or
    /// a grown person of their camp knows and they do not.
    ///
    /// How likely, by who they are and who teaches (#390): the day's chance
    /// is scaled by how readily the child learns and how readily the most
    /// willing of their teachers teaches (`Agent::how_readily_i_learn`,
    /// `how_readily_i_teach`), and the lesson comes from the most willing of
    /// those who know it. Each is the warmer to the other for it, and a proud
    /// teacher is pleased to pass on a thing they came to themselves.
    pub(in crate::analytics) fn the_young_are_taught(&mut self) {
        use rand::Rng;

        if self.current_turn % crate::environment::seasons::TICKS_PER_DAY != 0 {
            return;
        }
        let mut rng = crate::core::dice::roll();
        let mut lessons: Vec<(usize, usize, String)> = Vec::new();
        for (index, learner) in self.population.agents.iter().enumerate() {
            if !learner.state.is_alive
                || !matches!(learner.state.life_stage, LifeStage::Child | LifeStage::Adolescent)
            {
                continue;
            }
            let home = Self::where_this_one_lives(learner);
            let teachers: Vec<usize> = (0..self.population.agents.len())
                .filter(|&other| {
                    let teacher = &self.population.agents[other];
                    teacher.state.is_alive
                        && matches!(teacher.state.life_stage, LifeStage::Adult | LifeStage::Elderly)
                        && (learner.parent_ids.contains(&teacher.id)
                            || Self::within(
                                Self::where_this_one_lives(teacher),
                                home,
                                Self::NEAR_ENOUGH_TO_SLEEP_WITH_THE_OTHERS,
                            ))
                })
                .collect();
            let known_here: std::collections::BTreeSet<&String> = teachers
                .iter()
                .flat_map(|&teacher| self.population.agents[teacher].what_i_found_out().iter())
                .filter(|what| !learner.what_i_found_out().contains(*what))
                .collect();
            if known_here.is_empty() {
                continue;
            }
            let most_willing = teachers
                .iter()
                .map(|&teacher| self.population.agents[teacher].how_readily_i_teach())
                .fold(0.0, f64::max);
            let chance = Self::A_LESSON_A_DAY * learner.how_readily_i_learn() * most_willing;
            if !rng.gen_bool(chance.clamp(0.0, 1.0)) {
                continue;
            }
            let which = rng.gen_range(0..known_here.len());
            let Some(what) = known_here.into_iter().nth(which) else {
                continue;
            };
            let teacher = teachers
                .iter()
                .copied()
                .filter(|&teacher| self.population.agents[teacher].what_i_found_out().contains(what))
                .max_by(|&a, &b| {
                    self.population.agents[a]
                        .how_readily_i_teach()
                        .partial_cmp(&self.population.agents[b].how_readily_i_teach())
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            if let Some(teacher) = teacher {
                lessons.push((index, teacher, what.clone()));
            }
        }
        let now = self.current_turn;
        for (learner, teacher, what) in lessons {
            if !self.population.agents[learner].found_out_how_to(&what) {
                continue;
            }
            self.world.lessons_taught += 1;
            let (learner_id, teacher_id) = (self.population.agents[learner].id, self.population.agents[teacher].id);
            self.population.agents[learner]
                .relationships
                .get_or_create_relationship(teacher_id, now)
                .positive_interaction(Self::WHAT_A_LESSON_DOES_BETWEEN_THEM, now);
            let teacher = &mut self.population.agents[teacher];
            teacher
                .relationships
                .get_or_create_relationship(learner_id, now)
                .positive_interaction(Self::WHAT_A_LESSON_DOES_BETWEEN_THEM, now);
            if teacher.traits.has(crate::core::traits::Trait::Proud) && teacher.did_i_come_to_it_myself(&what) {
                let traits = teacher.traits.clone();
                teacher.emotions.add_happiness_with_traits(
                    crate::agents::EmotionSource::Event("passed_on_my_own".to_string()),
                    Self::WHAT_BEING_FIRST_IS_TO_THE_PROUD,
                    &traits,
                );
            }
        }
    }

    /// How much warmer a lesson makes teacher and taught to each other, on
    /// the relationship's own scale (#390).
    pub(in crate::analytics) const WHAT_A_LESSON_DOES_BETWEEN_THEM: i8 = 1;

    /// What coming to something first, or seeing it passed on, is worth to the
    /// proud, and what seeing somebody else come to it costs the envious
    /// (#390).
    pub(in crate::analytics) const WHAT_BEING_FIRST_IS_TO_THE_PROUD: f32 = 0.05;
    pub(in crate::analytics) const WHAT_ANOTHERS_FIND_IS_TO_THE_ENVIOUS: f32 = 0.05;

    /// What the camp makes of somebody coming to something themselves (#390):
    /// a proud finder is pleased with it, and the envious of their camp
    /// resent them for it.
    pub(in crate::analytics) fn what_the_camp_makes_of_it(&mut self) {
        use crate::core::traits::Trait;
        let finds: Vec<(usize, usize)> = self
            .population
            .agents
            .iter()
            .enumerate()
            .filter(|(_, agent)| agent.state.is_alive && !agent.lately_came_to.is_empty())
            .map(|(index, agent)| (index, agent.lately_came_to.len()))
            .collect();
        let now = self.current_turn;
        for (finder, how_many) in finds {
            self.population.agents[finder].lately_came_to.clear();
            let (finder_id, home) = {
                let agent = &mut self.population.agents[finder];
                if agent.traits.has(Trait::Proud) {
                    let traits = agent.traits.clone();
                    agent.emotions.add_happiness_with_traits(
                        crate::agents::EmotionSource::Event("came_to_it_first".to_string()),
                        Self::WHAT_BEING_FIRST_IS_TO_THE_PROUD * how_many as f32,
                        &traits,
                    );
                }
                (agent.id, Self::where_this_one_lives(agent))
            };
            for other in 0..self.population.agents.len() {
                let agent = &self.population.agents[other];
                if other == finder
                    || !agent.state.is_alive
                    || !agent.traits.has(Trait::Envious)
                    || !Self::within(Self::where_this_one_lives(agent), home, Self::NEAR_ENOUGH_TO_SLEEP_WITH_THE_OTHERS)
                {
                    continue;
                }
                let agent = &mut self.population.agents[other];
                let traits = agent.traits.clone();
                agent.emotions.add_sadness_with_traits(
                    crate::agents::EmotionSource::Event("another_came_to_it".to_string()),
                    Self::WHAT_ANOTHERS_FIND_IS_TO_THE_ENVIOUS * how_many as f32,
                    &traits,
                );
                agent
                    .relationships
                    .get_or_create_relationship(finder_id, now)
                    .negative_interaction(1, now);
            }
        }
    }
}
