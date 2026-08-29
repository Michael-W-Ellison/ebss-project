// src/analytics/between_us/asking.rs
//! Putting a question to somebody who might know.
//!
//! What asking would teach, and who is worth asking - which is not the same as
//! who is nearest, because a man who has never seen the thing cannot tell you
//! about it.
//!
//! Part of how one agent stands towards another - see [`super`].

use super::super::Simulation;

impl Simulation {
    /// A lump of clay left too near the fire.
    ///
    /// "An agent 'cooks' some clay which causes it to harden into stoneware,
    /// which unlocks that technology." Nobody intends this. Somebody is
    /// sitting at a fire with clay in their pack because they picked it up on
    /// the way past a riverbank, and a lump of it ends up in the embers, and
    /// in the morning it is not clay any more.
    ///
    /// The same shape as `who_saw_that_dry` and for the same reason: a people
    /// at this stage does not reason its way to firing clay, it notices that
    /// firing has happened. What it costs is one lump of clay; what it buys
    /// is the first material this people can make that keeps something else.
    /// What the person being asked could actually explain about this thing.
    ///
    /// They have to know it themselves - a man who has never dried anything
    /// cannot tell you how - and what passes is the name of the discovery
    /// rather than the thing. `None` where there is nothing to be said: most
    /// of what anybody carries is obvious, and nobody explains a stick.
    pub(in crate::analytics) fn what_asking_about_would_teach(&self, them: usize, what: &str) -> Option<String> {
        use crate::agents::Agent;

        let telling = &self.population.agents[them];

        // A meal that has been somewhere - dried, smoked - where what is worth
        // knowing is where it was rather than how it was made
        if let Some(item) = telling.inventory.get_item(what) {
            if let Some(discovery) = Agent::what_asking_about_this_meal_would_teach(item) {
                if telling.what_i_found_out().contains(discovery) {
                    return Some(discovery.to_string());
                }
            }
        }

        let made = Agent::what_asking_about_this_would_teach(what)?;

        telling.what_i_found_out().contains(&made).then_some(made)
    }

    /// Somebody near enough to ask, and a thing of theirs worth asking about.
    ///
    /// Worth asking about means: they are carrying it, this one has never seen
    /// how it is done, and they can actually explain it. Nobody asks after a
    /// stick.
    ///
    /// This is only ever reached under Curiosity, which is to say only when
    /// nothing worse is pressing - a man does not stop to ask after somebody's
    /// supper while his own children are hungry.
    pub(in crate::analytics) fn somebody_to_ask_about_something(
        &self,
        agent: &crate::agents::Agent,
        agent_position: (i32, i32, i32),
    ) -> Option<(uuid::Uuid, String)> {
        for (index, other) in self.population.agents.iter().enumerate() {
            if !other.state.is_alive || other.id == agent.id {
                continue;
            }

            let apart = (other.state.position.0 - agent_position.0)
                .abs()
                .max((other.state.position.1 - agent_position.1).abs());

            if apart > Self::NEAR_ENOUGH_TO_ASK {
                continue;
            }

            for (item_id, _) in other.inventory.get_all_items().iter() {
                let Some(teaches) = self.what_asking_about_would_teach(index, item_id) else {
                    continue;
                };

                if agent.what_i_found_out().contains(&teaches) {
                    continue;
                }

                return Some((other.id, item_id.clone()));
            }
        }

        None
    }

    /// How near somebody has to be to be asked.
    pub(in crate::analytics) const NEAR_ENOUGH_TO_ASK: i32 = 2;
}
