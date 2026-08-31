// src/analytics/happening/senses.rs
//! What can be smelled, and what stops being worth remembering.
//!
//! Part of what happens whether or not anybody decides anything - see
//! [`super`]. Called from [`crate::analytics::turn`], in the order argued over
//! there.

use super::super::Simulation;

impl Simulation {
    /// Clean ground within a step or two, for somebody standing on a midden.
    ///
    /// `None` when the ground underfoot is fine, which is the ordinary case
    /// and costs one lookup.
    pub(in crate::analytics) fn somewhere_that_does_not_stink(
        &self,
        from: (i32, i32, i32),
    ) -> Option<(i32, i32, i32)> {
        use crate::world::Position;

        let here = Position::new(from.0, from.1);
        let underfoot = self.world.grid.get_tile(&here)?;
        if !underfoot.soil.is_foul() {
            return None;
        }

        let mut best: Option<((i32, i32, i32), f32)> = None;

        for dy in -Self::OFF_THE_MIDDEN..=Self::OFF_THE_MIDDEN {
            for dx in -Self::OFF_THE_MIDDEN..=Self::OFF_THE_MIDDEN {
                if dx == 0 && dy == 0 {
                    continue;
                }

                let there = Position::new(from.0 + dx, from.1 + dy);
                let Some(tile) = self.world.grid.get_tile(&there) else {
                    continue;
                };
                if tile.soil.is_foul() || !tile.terrain.is_walkable() {
                    continue;
                }

                // The nearest clean tile, so that this is a step aside rather
                // than a march.
                let how_far = (dx.abs() + dy.abs()) as f32;
                if best.is_none_or(|(_, best_so_far)| how_far < best_so_far) {
                    best = Some(((there.x, there.y, from.2), how_far));
                }
            }
        }

        best.map(|(where_it_is, _)| where_it_is)
    }

    /// Emit the smells of the world to agents in range.
    ///
    /// Resource percepts are only ever derived from smell, so without this the
    /// agents never perceive resources at all. What carries how far is
    /// deliberate: a human nose is poor, and finds food mainly when the food is
    /// cooking or rotting. Agents find whole, raw food by looking instead.
    ///
    /// Three things give themselves away:
    /// - what lies on the ground, faintly, and mostly if it is flesh
    /// - food that has turned, wherever it is being carried
    /// - a lit fire with something in it, which carries furthest of all
    /// Take food off the fire once it has had its time there.
    ///
    /// The heat sources were built for smelting, where contents sit until a
    /// recipe consumes them. Food has no such recipe, so without this a fire
    /// that once had a meal on it would smell of cooking for the rest of the
    /// run.
    pub(in crate::analytics) fn clear_finished_cooking(&mut self) {
        let cooking_time = Self::COOKING_SMELL_TICKS;

        for heat_source in self.world.heat_sources.all_mut() {
            heat_source.contents.retain(|content| {
                let is_food = crate::agents::storage_integration::id_to_item_type(
                    &content.material_id,
                )
                .map(|item_type| {
                    item_type.cooking_outcome() != crate::world::nutrition::CookingOutcome::NotFood
                })
                .unwrap_or(false);

                !is_food || content.heating_time < cooking_time
            });
        }
    }

    pub(in crate::analytics) fn emit_scents(&mut self) {
        use crate::agents::senses::{Scent, ScentType};

        let sources = self.collect_scent_sources();

        for agent in &mut self.population.agents {
            if !agent.state.is_alive {
                continue;
            }

            let agent_pos = agent.state.position;

            // Scents are re-derived from the world every tick, so the previous
            // set is dropped first. Appending instead would pile up thousands
            // of duplicates, and stale ones would keep rebuilding memories of
            // patches that no longer exist.
            agent.senses.smell.detected_scents.retain(|scent| {
                !matches!(
                    scent.scent_type,
                    ScentType::Food | ScentType::Water | ScentType::Decay
                )
            });

            for (source_position, scent_type, strength) in &sources {
                if agent.senses.smell.can_smell(agent_pos, *source_position, *strength) {
                    agent.senses.smell.detect_scent(Scent {
                        source_position: *source_position,
                        scent_type: scent_type.clone(),
                        strength: *strength,
                        age: 0,
                    });
                }
            }
        }
    }

    /// Everything in the world currently giving off a smell
    pub(in crate::analytics) fn collect_scent_sources(
        &self,
    ) -> Vec<((i32, i32, i32), crate::agents::senses::ScentType, f32)> {
        use crate::agents::senses::ScentType;
        use crate::world::ResourceType;

        let mut sources = Vec::new();

        // What lies on the ground. Berries on the bush are close to odourless,
        // so an agent finds those by looking rather than by sniffing.
        for resource in &self.world.resources {
            if resource.amount == 0 {
                continue;
            }

            let strength = resource.resource_type.raw_scent_strength();
            if strength <= 0.0 {
                continue;
            }

            let scent_type = if resource.resource_type == ResourceType::Water {
                ScentType::Water
            } else {
                ScentType::Food
            };

            sources.push((
                (resource.position.x, resource.position.y, 0),
                scent_type,
                strength,
            ));
        }

        // Food that has turned announces itself, wherever it is being carried.
        // This is decay rather than food: it says something is rotten here, and
        // does not send an agent over to eat it.
        for agent in &self.population.agents {
            if !agent.state.is_alive {
                continue;
            }

            let rot = agent
                .inventory
                .get_all_items()
                .iter()
                .filter_map(|(_, item)| item.food_data.as_ref())
                .filter(|food| food.is_rotting() || food.is_ruined())
                .map(|food| food.scent_strength())
                .fold(0.0_f32, f32::max);

            if rot > 0.0 {
                sources.push((agent.state.position, ScentType::Decay, rot));
            }
        }

        // A midden. "Waste should smell unpleasant and repulse the agents":
        // this is the smell of it. It reaches further than a berry does and
        // nowhere near as far as a cooking fire, which is about right for
        // something you notice when you are nearly standing in it.
        // The ground that has something on it, rather than every tile in the
        // world - see `Grid::note_something_on`. Muck only ever arrives
        // through `somebody_voided_here`, which notes the tile as it lands.
        for at in self.world.grid.where_the_ground_is_doing_something() {
            let Some(tile) = self.world.grid.get_tile(&at) else {
                continue;
            };

            if !tile.soil.is_foul() {
                continue;
            }

            let strength = (tile.soil.fouling
                / crate::world::soil::Soil::AS_FOUL_AS_IT_GETS)
                .clamp(0.0, 1.0);
            sources.push(((at.x, at.y, 0), ScentType::Decay, strength));
        }

        // A lit fire with food in it: the strongest smell there is, and the one
        // a nose is really for.
        //
        // Nothing lights a fire or puts food in one yet, so this source is
        // dormant in a live run - see ISSUES_FOUND.md.
        for heat_source in self.world.heat_sources.all() {
            if !heat_source.is_lit || heat_source.contents.is_empty() {
                continue;
            }

            sources.push((heat_source.position, ScentType::Food, 1.0));
        }

        sources
    }

    /// Drop food memories near the agent after a fruitless search there.
    ///
    /// Resource nodes are removed once exhausted, so an agent that walks to a
    /// remembered berry patch and finds nothing would otherwise keep walking
    /// back to the same empty spot until it starved.
    pub(in crate::analytics) fn forget_nearby_food_memories(&mut self, agent_index: usize) {
        use crate::core::memory::SpatialMemoryType;

        let agent = &mut self.population.agents[agent_index];
        let pos = agent.state.position;

        let stale: Vec<(i32, i32, i32)> = agent
            .memory
            .recall_locations(SpatialMemoryType::Food)
            .into_iter()
            .map(|memory| memory.position)
            .filter(|remembered| {
                (remembered.0 - pos.0).abs() + (remembered.1 - pos.1).abs() <= 3
            })
            .collect();

        for position in stale {
            agent.memory.forget_location(SpatialMemoryType::Food, position);
        }
    }
}
