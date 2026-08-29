// src/analytics/between_us/seeing.rs
//! What everybody saw, and what they made of it.
//!
//! Sight is the only channel in this model that reaches a whole settlement in
//! one tick, which is why a wolf on the ridge frightens more people than the
//! one man who met it.
//!
//! Part of how one agent stands towards another - see [`super`].

use super::super::Simulation;

impl Simulation {
    /// What everybody can see that frightens them, and where it was.
    ///
    /// The map an agent carries had explored tiles, resource positions with
    /// an age and a source, buildings, storage and terrains - a real picture
    /// of the world's *things* - and nothing at all about danger. Somebody
    /// could be mauled at a ford and walk back to the same ford the next
    /// morning, because there was nowhere for "there are wolves in that wood"
    /// to live.
    ///
    /// This is the sight pass for it. What goes in is what an agent would
    /// actually notice: a beast within sight that means it harm, and how the
    /// odds looked at the time. Reading the odds rather than the species is
    /// what stops a man with a spear being as frightened of a wolf as a child
    /// with nothing.
    pub(in crate::analytics) fn what_everybody_saw_that_frightened_them(&mut self) {
        if self.current_tick % Self::HOW_OFTEN_ANYBODY_LOOKS_ROUND != 0 {
            return;
        }

        let now = self.current_tick;

        // Everything alive that means anybody harm, with what it is worth in
        // a fight and what to call it
        let beasts: Vec<((i32, i32), f32, f32, String)> = self
            .world
            .animals
            .get_all()
            .iter()
            .filter(|animal| animal.is_alive())
            .filter_map(|animal| {
                let species = self.world.animals.get_species(&animal.species_id)?;
                let menace = species.behavior.how_much_it_menaces_you();
                if menace <= 0.0 {
                    return None;
                }
                let worth = Self::what_a_beast_is_worth_in_a_fight(
                    animal.current_health,
                    species.health,
                    species.attack_damage,
                );
                Some((animal.position, worth, menace, species.name.clone()))
            })
            .collect();

        if beasts.is_empty() {
            return;
        }

        for agent in self.population.agents.iter_mut() {
            if !agent.state.is_alive {
                continue;
            }

            let armed = agent
                .what_i_have_to_work_with(crate::agents::SkillType::MeleeCombat)
                .is_some();
            let i_am_worth = Self::WHAT_A_PERSON_IS_WORTH_TO_A_BEAST
                * if armed { Self::WHAT_A_SPEAR_ADDS } else { 1.0 };

            // Everything in sight that means harm, taken together.
            //
            // Together rather than one at a time, because that is what the
            // specification says a threat is: "a man encountering 4 wolves
            // should see them as a threat". One wolf is not much to a man
            // with a spear and four of them are a different afternoon
            // entirely, and judging each separately would have him walk into
            // the pack four times unafraid.
            let in_sight: Vec<&((i32, i32), f32, f32, String)> = beasts
                .iter()
                .filter(|((x, y), _, _, _)| {
                    (agent.state.position.0 - x)
                        .abs()
                        .max((agent.state.position.1 - y).abs())
                        <= Self::AS_FAR_AS_ANYBODY_SEES_A_BEAST
                })
                .collect();

            if in_sight.is_empty() {
                continue;
            }

            let against_me: f32 = in_sight
                .iter()
                .map(|(_, worth, menace, _)| worth * menace)
                .sum();

            // A thing worth twice what you are is frightening; a thing worth
            // half of you is not worth remembering.
            let odds = against_me / i_am_worth.max(0.01);
            let how_bad = (odds - 1.0).clamp(0.0, 1.0);

            if how_bad <= 0.0 {
                continue;
            }

            // What to call it is whatever the worst single one of them was.
            let called = in_sight
                .iter()
                .max_by(|(_, one, _, _), (_, other, _, _)| {
                    one.partial_cmp(other).unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|(_, _, _, called)| called.clone())
                .unwrap_or_else(|| "something".to_string());

            for ((x, y), _, _, _) in in_sight {
                agent.exploration_knowledge.saw_danger(
                    crate::world::Position::new(*x, *y),
                    &called,
                    how_bad,
                    now,
                );
            }
        }
    }

    /// And whoever anybody laid eyes on, and where.
    ///
    /// Everything social in the model reads live positions, which is to say
    /// every agent knows where every other agent is standing at all times.
    /// This is what somebody would actually know.
    pub(in crate::analytics) fn who_everybody_saw(&mut self) {
        if self.current_tick % Self::HOW_OFTEN_ANYBODY_LOOKS_ROUND != 0 {
            return;
        }

        let now = self.current_tick;
        let standing: Vec<(uuid::Uuid, (i32, i32))> = self
            .population
            .agents
            .iter()
            .filter(|agent| agent.state.is_alive)
            .map(|agent| (agent.id, (agent.state.position.0, agent.state.position.1)))
            .collect();

        for agent in self.population.agents.iter_mut() {
            if !agent.state.is_alive {
                continue;
            }

            for (who, (x, y)) in standing.iter() {
                if *who == agent.id {
                    continue;
                }

                let paces = (agent.state.position.0 - x)
                    .abs()
                    .max((agent.state.position.1 - y).abs());

                if paces <= Self::AS_FAR_AS_ANYBODY_SEES_A_PERSON {
                    agent.exploration_knowledge.saw_somebody(
                        *who,
                        crate::world::Position::new(*x, *y),
                        now,
                    );
                }
            }
        }
    }

    /// How often anybody stops and takes in what is round them.
    ///
    /// Every few ticks rather than every one. Nothing in a settlement changes
    /// fast enough to want it more often, and it is a walk over everybody
    /// against everything.
    pub(in crate::analytics) const HOW_OFTEN_ANYBODY_LOOKS_ROUND: u32 = 5;

    /// How far off a beast is worth noticing.
    pub(in crate::analytics) const AS_FAR_AS_ANYBODY_SEES_A_BEAST: i32 = 8;

    /// And a person, who is smaller and quieter than a bear.
    pub(in crate::analytics) const AS_FAR_AS_ANYBODY_SEES_A_PERSON: i32 = 6;
}
