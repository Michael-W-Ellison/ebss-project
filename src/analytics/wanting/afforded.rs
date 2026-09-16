// src/analytics/wanting/afforded.rs
//! What could I do here, holding this?
//!
//! The verb matrix has always known the answer and has never been asked.
//! `EVERY_VERB` declares, for each of seventy-four verbs, what it acts on and
//! what it wants in the hands doing it - and outside the tests the only
//! production caller of the module is `errands`, which asks
//! `what_this_action_cannot_do_without(named)`: *what does this
//! already-chosen action want, so I can go and fetch it*. The matrix is
//! consulted about a verb the hand-written gates have already picked. Nobody
//! ever asked it the other way round.
//!
//! This is the other way round. Run the same two tests - is there such a
//! target here, will these hands do - across the whole matrix instead of
//! across one named verb, and what comes back is the set of things this
//! person could do, standing here, carrying what they are carrying.
//!
//! **That makes "holding a thing opens new verbs" true by construction rather
//! than by anybody writing it down.** A man with a free hand and a stone
//! underfoot can `stack`; put the stone in his hand and he can `throw`. Both
//! verbs have been in the matrix all along with their wants declared; what
//! was missing was anything that enumerated them. Nothing here knows what a
//! stone is for, and that is the point - the affordance falls out of the
//! matrix, so a new thing with the right tags inherits the verb surface
//! without a line of code.
//!
//! What this deliberately does **not** do is choose. Ranking these candidates
//! is the novelty question - how often has this verb been tried on this kind
//! of thing, and did it ever come to anything - and it wants the per-verb,
//! per-object lesson key to exist first. That is a separate piece of work and
//! this is the generator it will read.

use super::super::Simulation;
use crate::agents::Agent;
use crate::environment::verbs::{Targets, Verb, EVERY_VERB};
use crate::world::Position;

impl Simulation {
    /// How far off a thing can be and still be something you could act on.
    ///
    /// A person or an animal, not a bush: the verbs that target those are
    /// hand-to-hand, and a deer across the valley is an expedition rather than
    /// an affordance. See `AS_NEAR_AS_PREY_HAS_TO_BE_TO_BOTHER`, which is the
    /// same argument made about hunting.
    pub(in crate::analytics) const WITHIN_REACH: f32 = 1.5;

    /// Every verb this agent could perform standing where it is.
    ///
    /// Includes verbs the matrix declares and nothing performs - `done_by` is
    /// `None` for eighteen of them, `stack` and `throw` among them. They are
    /// returned rather than filtered out on purpose: a generator that quietly
    /// dropped them would report a world of possibilities that is smaller than
    /// the matrix says and give no sign of the difference. `what_i_could_do_here_now`
    /// is the filtered form, for a caller that means to act.
    pub fn what_i_could_do_here(&self, agent: &Agent) -> Vec<&'static Verb> {
        EVERY_VERB
            .iter()
            .filter(|verb| self.is_there_such_a_target(verb.targets, agent))
            .filter(|verb| Self::do_these_hands_do(agent, &verb.wants))
            .collect()
    }

    /// The same, less the verbs nothing in the simulation carries out.
    pub fn what_i_could_do_here_now(&self, agent: &Agent) -> Vec<&'static Verb> {
        self.what_i_could_do_here(agent)
            .into_iter()
            .filter(|verb| verb.done_by.is_some())
            .collect()
    }

    /// Whether there is anything here for a verb of this sort to act on.
    ///
    /// The half of the matrix nothing has ever answered. `Wants` came with
    /// `satisfied_by_hands` and so could always be asked; `Targets` was
    /// declared, tested for coverage, and never once put to a world.
    pub fn is_there_such_a_target(&self, targets: Targets, agent: &Agent) -> bool {
        let at = agent.state.position;
        let here = Position::new(at.0, at.1);

        match targets {
            // Nothing outside the actor, so there is always such a target:
            // somebody can always sit down.
            Targets::Nobody => true,

            // Somewhere else on the map. There always is somewhere else.
            Targets::APlace => true,

            Targets::AThingHeld => !agent.inventory.get_all_items().is_empty(),

            // Lying here, growing here, or buried here. A pit counts: what is
            // in a store is as much underfoot as what is on the grass, and the
            // store branch of `picking_up` has always treated it so.
            Targets::AThingUnderfoot => {
                !self.world.what_is_lying_at(&here).is_empty()
                    || self
                        .world
                        .get_resource_at(&here)
                        .is_some_and(|node| node.amount > 0)
                    || self.world.pit_at(here).is_some()
            }

            // The tile itself - which exists unless the agent is off the map,
            // and an agent off the map has worse problems than this.
            Targets::TheGroundUnderfoot => self.world.grid.get_tile(&here).is_some(),

            Targets::APerson => self.population.agents.iter().any(|other| {
                other.state.is_alive
                    && other.id != agent.id
                    && Self::within_reach(other.state.position, at)
            }),

            Targets::AnAnimal => !self
                .world
                .get_animals_in_radius((at.0, at.1), Self::WITHIN_REACH)
                .is_empty(),

            Targets::AFire => self.world.get_heat_source_at(at.0, at.1).is_some(),

            Targets::AStructure => self.world.get_building_at(&here).is_some(),

            // Water to act on: a river or a pool underfoot, or a spring on
            // this tile. Not what is in a skin - a verb wanting that says so
            // through `Wants::AVessel`, which is the hands half of the
            // question and already answered there.
            Targets::Water => {
                self.world
                    .grid
                    .get_tile(&here)
                    .is_some_and(|tile| tile.terrain.terrain_type == crate::world::TerrainType::Water)
                    || self.world.get_resource_at(&here).is_some_and(|node| {
                        node.resource_type == crate::world::ResourceType::Water && node.amount > 0
                    })
            }
        }
    }

    /// Whether two people are near enough for one to act on the other.
    fn within_reach(one: (i32, i32, i32), other: (i32, i32, i32)) -> bool {
        let paces = (one.0 - other.0).abs().max((one.1 - other.1).abs());
        paces as f32 <= Self::WITHIN_REACH
    }
}
