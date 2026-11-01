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

use super::super::Simulation;
use crate::agents::LifeStage;

impl Simulation {
    /// The chance, a day, that a child learns one more thing the grown
    /// people about them know: a thing a month, so a childhood takes in a
    /// hundred or more.
    pub(in crate::analytics) const A_LESSON_A_DAY: f64 = 1.0 / 30.0;

    /// Once a day, each child and adolescent may learn one thing a parent or
    /// a grown person of their camp knows and they do not.
    pub(in crate::analytics) fn the_young_are_taught(&mut self) {
        use rand::Rng;

        if self.current_turn % crate::environment::seasons::TICKS_PER_DAY != 0 {
            return;
        }
        let mut rng = crate::core::dice::roll();
        let mut lessons: Vec<(usize, String)> = Vec::new();
        for (index, learner) in self.population.agents.iter().enumerate() {
            if !learner.state.is_alive
                || !matches!(learner.state.life_stage, LifeStage::Child | LifeStage::Adolescent)
            {
                continue;
            }
            let home = Self::where_this_one_lives(learner);
            let known_here: std::collections::BTreeSet<&String> = self
                .population
                .agents
                .iter()
                .filter(|teacher| {
                    teacher.state.is_alive
                        && matches!(teacher.state.life_stage, LifeStage::Adult | LifeStage::Elderly)
                        && (learner.parent_ids.contains(&teacher.id)
                            || Self::within(
                                Self::where_this_one_lives(teacher),
                                home,
                                Self::NEAR_ENOUGH_TO_SLEEP_WITH_THE_OTHERS,
                            ))
                })
                .flat_map(|teacher| teacher.what_i_found_out().iter())
                .filter(|what| !learner.what_i_found_out().contains(*what))
                .collect();
            if known_here.is_empty() || !rng.gen_bool(Self::A_LESSON_A_DAY) {
                continue;
            }
            let which = rng.gen_range(0..known_here.len());
            if let Some(what) = known_here.into_iter().nth(which) {
                lessons.push((index, what.clone()));
            }
        }
        for (index, what) in lessons {
            if self.population.agents[index].found_out_how_to(&what) {
                self.world.lessons_taught += 1;
            }
        }
    }
}
