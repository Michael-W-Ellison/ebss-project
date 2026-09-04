// src/analytics/doing/moving.rs
//! Going somewhere, or staying put.
//!
//! Walking, seeking shelter, mounting and dismounting, exploring, sleeping
//! and waiting.
//!
//! One method per `Action` variant, called from the dispatcher in
//! [`super::execute_action`]. The bodies are as they were when all fifty-two
//! lived in one five-thousand-line `match`; what changed is that a verb can
//! now be found, read and altered without scrolling past the other fifty-one.

use super::super::Simulation;
use crate::core::DriveType;
use crate::environment::ActionResult;
use log::debug;
use rand::Rng;

impl Simulation {
    /// `Action::Sleep`.
    pub(in crate::analytics) fn sleeping(&mut self, duration: &u32, agent_index: usize) -> ActionResult {
        let current_tick = self.current_tick;
        let has_shelter = self.agent_has_shelter(agent_index);
        let agent = &mut self.population.agents[agent_index];

        // Sleep quality depends on the agent's circumstances
        let quality_factors = crate::agents::fatigue::SleepQualityFactors {
            has_shelter,
            has_bed: has_shelter,
            safety: 1.0 - agent.emotions.fear.min(1.0),
            health: (agent.state.health / 100.0).clamp(0.0, 1.0),
            hunger: agent
                .drives
                .get(DriveType::Hunger)
                .map(|d| d.value)
                .unwrap_or(0.0),
            comfort: 0.5,
        };

        // Actually recover fatigue rather than only topping up energy;
        // without this the agent's fatigue never falls, so an exhausted
        // agent re-selects Sleep every tick and never does anything else.
        let energy_before = agent.state.energy;
        let mut fatigue_recovered = 0.0;
        for _ in 0..(*duration).max(1) {
            fatigue_recovered += agent.sleep_tick(current_tick, &quality_factors);
        }
        agent.wake_up(current_tick);

        let energy_restored = agent.state.energy - energy_before;

        ActionResult::success()
            .with_drive_change(DriveType::Rest, -0.5)
            .with_message(format!(
                "Slept for {} ticks, recovered {:.2} fatigue and {:.1} energy",
                duration, fatigue_recovered, energy_restored
            ))
    }

    /// `Action::Move`.
    pub(in crate::analytics) fn walking(&mut self, target: &(i32, i32, i32), agent_index: usize) -> ActionResult {
        use crate::world::grid::Position;

        // Get agent current position
        let agent = &self.population.agents[agent_index];
        let current_pos = agent.state.position;
        let current_2d = Position::new(current_pos.0, current_pos.1);

        // Target position
        let target_2d = Position::new(target.0, target.1);

        // Check if already at target (including Z-axis)
        if current_2d == target_2d && current_pos.2 == target.2 {
            return ActionResult::success()
                .with_message("Already at destination".to_string());
        }

        // Calculate movement distance (3D Manhattan distance)
        let dx = target.0 - current_pos.0;
        let dy = target.1 - current_pos.1;
        let dz = target.2 - current_pos.2;

        // Normalize to -1, 0, or 1 for each axis
        let step_x = if dx > 0 { 1 } else if dx < 0 { -1 } else { 0 };
        let step_y = if dy > 0 { 1 } else if dy < 0 { -1 } else { 0 };
        let step_z = if dz > 0 { 1 } else if dz < 0 { -1 } else { 0 };

        // Determine next step - prioritize horizontal movement, then vertical
        // This models climbing/descending as slower than horizontal movement
        //
        // Candidates are ordered best-first: the direct step, then the
        // other axis, then a sidestep. Without the fallbacks an agent
        // whose direct route runs into a lake retries the same blocked
        // step forever, which strands it (and, if it was on its way to
        // food, starves it).
        let mut candidates: Vec<(i32, i32, i32)> = Vec::new();
        let push = |candidate: (i32, i32, i32), candidates: &mut Vec<(i32, i32, i32)>| {
            if candidate != current_pos && !candidates.contains(&candidate) {
                candidates.push(candidate);
            }
        };

        let x_step = (current_pos.0 + step_x, current_pos.1, current_pos.2);
        let y_step = (current_pos.0, current_pos.1 + step_y, current_pos.2);
        let z_step = (current_pos.0, current_pos.1, current_pos.2 + step_z);

        if dx.abs() >= dy.abs() && dx.abs() >= dz.abs() {
            push(x_step, &mut candidates);
            push(y_step, &mut candidates);
            push(z_step, &mut candidates);
        } else if dy.abs() >= dz.abs() {
            push(y_step, &mut candidates);
            push(x_step, &mut candidates);
            push(z_step, &mut candidates);
        } else {
            push(z_step, &mut candidates);
            push(x_step, &mut candidates);
            push(y_step, &mut candidates);
        }

        // Sidesteps perpendicular to the blocked direction, so agents
        // can work their way around an obstacle rather than stall
        for (side_x, side_y) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            push(
                (current_pos.0 + side_x, current_pos.1 + side_y, current_pos.2),
                &mut candidates,
            );
        }

        // Take the direct step when it is clear; otherwise search for a
        // route around whatever is in the way before falling back to a
        // sidestep, so agents do not oscillate against an obstacle.
        let direct_step = candidates
            .first()
            .copied()
            .filter(|candidate| self.is_passable_tile(candidate.0, candidate.1));

        let step = direct_step
            .or_else(|| self.next_step_toward(current_pos, *target))
            .or_else(|| {
                candidates
                    .iter()
                    .copied()
                    .find(|candidate| self.is_passable_tile(candidate.0, candidate.1))
            });

        let (next_x, next_y, next_z) = match step {
            Some(step) => step,
            None => {
                return ActionResult::failure(
                    "No passable route toward destination".to_string(),
                )
            }
        };

        // Get movement speed multiplier from leg health
        let agent = &self.population.agents[agent_index];
        let body_speed = agent.body.movement_speed_multiplier();

        // Get transport speed multiplier (mounts provide speed boost!)
        let transport_speed = agent.transport.effective_speed_modifier();

        // Combined movement speed (body health * transport bonus)
        let movement_speed = body_speed * transport_speed;

        // Base energy cost (modified by speed and distance)
        let base_energy_cost = 2.0;
        let actual_energy_cost = if movement_speed > 0.1 {
            // And by what is on the agent's back. Carrying was free
            // until now: a man walked as easily under sixty pounds of
            // stone as under nothing, which made a full pack pure
            // gain and a basket a thing with no cost at all. A load
            // is paid for with every step taken under it - which is
            // what `verbs::CARRY` had always claimed and nothing had
            // ever made true.
            base_energy_cost * Self::what_this_load_costs(agent) / movement_speed
        } else {
            // Legs too damaged, can't move
            return ActionResult::failure("Too injured to move (legs crippled)".to_string());
        };

        // Update agent position (including Z-axis)
        let agent = &mut self.population.agents[agent_index];
        agent.state.position = (next_x, next_y, next_z);

        // Calculate remaining 3D distance
        let remaining_distance = ((target.0 - next_x).abs() + (target.1 - next_y).abs() + (target.2 - next_z).abs()) as u32;

        debug!(
            "Agent {} moved from ({}, {}, {}) to ({}, {}, {}) (distance to target: {}, speed: {:.2}x, mounted: {})",
            agent.id, current_pos.0, current_pos.1, current_pos.2, next_x, next_y, next_z,
            remaining_distance, movement_speed,
            if agent.transport.is_mounted() { "yes" } else { "no" }
        );

        // Determine drive satisfaction based on purpose (Safety or Curiosity)
        let drive_type = if remaining_distance <= 5 {
            Some(DriveType::Safety) // Moving to nearby location (fleeing or seeking safety)
        } else {
            Some(DriveType::Curiosity) // Exploring distant location
        };

        let mut result = ActionResult::success()
            .with_energy_cost(actual_energy_cost)
            .with_message(format!("Moved to ({}, {}, {}), {} steps to goal", next_x, next_y, next_z, remaining_distance));

        if let Some(drive) = drive_type {
            result = result.with_drive_change(drive, -0.05);
        }

        result
    }

    /// `Action::SeekShelter`.
    pub(in crate::analytics) fn seeking_shelter(&mut self, agent_index: usize) -> ActionResult {
        // Find nearest shelter (completed building or forest)
        let agent_tuple_pos = self.population.agents[agent_index].state.position;
        let agent_pos = crate::world::Position::new(agent_tuple_pos.0, agent_tuple_pos.1);

        // Check if already in shelter
        let in_building = self.world.buildings.iter().any(|b| {
            b.position == agent_pos && b.is_completed()
        });

        let in_forest = self.world.grid.get_tile(&agent_pos)
            .map(|t| matches!(t.terrain.terrain_type, crate::world::TerrainType::Forest))
            .unwrap_or(false);

        if in_building || in_forest {
            // Already in shelter - recover from exposure
            let agent = &mut self.population.agents[agent_index];
            agent.exposure_status.recover(0.05);

            return ActionResult::success()
                .with_drive_change(DriveType::Safety, -0.3)
                .with_energy_cost(0.0)
                .with_message(format!(
                    "Taking shelter (exposure: {:.2})",
                    agent.exposure_status.exposure_damage
                ));
        }

        let nearest_shelter = self.nearest_shelter_from(agent_tuple_pos);

        // Move towards nearest shelter, routing around obstacles.
        // Stepping straight at it stalls against the first lake or
        // building in the way, which strands the agent in the weather
        // it was trying to escape.
        if let Some(shelter_pos) = nearest_shelter {
            let target = (shelter_pos.x, shelter_pos.y, agent_tuple_pos.2);

            match self.next_step_toward(agent_tuple_pos, target) {
                Some(step) => {
                    let agent = &mut self.population.agents[agent_index];
                    agent.state.position = step;

                    ActionResult::success()
                        .with_drive_change(DriveType::Safety, -0.1)
                        .with_energy_cost(5.0)
                        .with_message(format!(
                            "Moving towards shelter at ({}, {})",
                            shelter_pos.x, shelter_pos.y
                        ))
                }
                None => ActionResult::failure("Path to shelter blocked".to_string()),
            }
        } else {
            ActionResult::failure("No shelter found nearby".to_string())
        }
    }

    /// `Action::Mount`.
    pub(in crate::analytics) fn mounting(&mut self, transport_id: &uuid::Uuid, agent_index: usize) -> ActionResult {
        let agent = &mut self.population.agents[agent_index];

        // Try to mount the transport
        match agent.transport.mount_transport(transport_id) {
            Ok(()) => {
                debug!("Agent {} mounted transport {}", agent.id, transport_id);

                ActionResult::success()
                    .with_drive_change(DriveType::Utility, -0.1)
                    .with_energy_cost(2.0)
                    .with_message("Successfully mounted".to_string())
            }
            Err(err) => ActionResult::failure(err),
        }
    }

    /// `Action::Dismount`.
    pub(in crate::analytics) fn dismounting(&mut self, agent_index: usize) -> ActionResult {
        let agent = &mut self.population.agents[agent_index];

        if !agent.transport.is_mounted() {
            return ActionResult::failure("Not currently mounted".to_string());
        }

        agent.transport.dismount_current();
        debug!("Agent {} dismounted", agent.id);

        ActionResult::success()
            .with_energy_cost(1.0)
            .with_message("Dismounted from transport".to_string())
    }

    /// `Action::Wait`.
    pub(in crate::analytics) fn waiting(&mut self, agent_index: usize, rng: &mut rand::rngs::StdRng) -> ActionResult {
        // Wait/rest action - restores energy, calms emotions
        let agent = &mut self.population.agents[agent_index];

        // Restore a small amount of energy (resting)
        let energy_restored = rng.gen_range(3.0..6.0);
        agent.state.energy = (agent.state.energy + energy_restored).min(100.0);

        // Reduce negative emotions slightly (calming effect)
        agent.emotions.anger = (agent.emotions.anger - 0.02).max(0.0);
        agent.emotions.fear = (agent.emotions.fear - 0.02).max(0.0);

        debug!(
            "Agent {} waited, restored {:.1} energy, reduced stress",
            agent.id, energy_restored
        );

        ActionResult::success()
            .with_drive_change(DriveType::Rest, -0.15) // Satisfies rest drive
            .with_message(format!("Rested and recovered {:.1} energy", energy_restored))
    }

    /// `Action::Explore`.
    pub(in crate::analytics) fn exploring(&mut self, direction: &(i32, i32, i32), agent_index: usize, tick_now: u32) -> ActionResult {
        // Exploration action - move and discover new areas
        let current_pos = self.population.agents[agent_index].state.position;

        // Calculate target position in exploration direction
        let target_x = current_pos.0 + direction.0;
        let target_y = current_pos.1 + direction.1;
        let target_z = current_pos.2 + direction.2;
        let target_pos = (target_x, target_y, target_z);

        // What is really out here, before anybody's opinion of it
        let exploration_radius = 3; // Can see 3 tiles in each direction
        let really_here: std::collections::BTreeSet<crate::world::Position> = self
            .world
            .resources
            .iter()
            .filter(|resource| {
                (resource.position.x - target_x).abs() <= exploration_radius
                    && (resource.position.y - target_y).abs() <= exploration_radius
            })
            .map(|resource| {
                crate::world::Position::new(resource.position.x, resource.position.y)
            })
            .collect();

        let worked_out = &self.world.where_it_was_worked_out;

        let agent = &mut self.population.agents[agent_index];
        let agent_id = agent.id;

        // Move agent to new position
        agent.state.position = target_pos;

        // Mark tiles as explored in a radius around new position
        let mut newly_explored_count = 0;

        for dx in -exploration_radius..=exploration_radius {
            for dy in -exploration_radius..=exploration_radius {
                let explore_pos = crate::world::Position::new(
                    target_x + dx,
                    target_y + dy,
                );

                if agent.exploration_knowledge.explore_tile(explore_pos, self.current_tick) {
                    newly_explored_count += 1;
                }
            }
        }

        // And the area takes an impression. Once a day, however long anybody
        // stands in it - see `agents::whereabouts`. This is the general half
        // of the map: what a place leaves behind by being lived in, as
        // against the places that answered something, which are kept for
        // years and separately.
        agent.whereabouts.looked_at(
            crate::agents::whereabouts::Area::holding(target_pos),
            crate::agents::Agent::what_day_it_is(self.current_tick),
        );

        // Seeing for yourself.
        //
        // An agent's knowledge of where things are is fed both by
        // looking and by being told, and the two went into the same
        // map with nothing to tell them apart. So a man walked to the
        // place he had been told about, found bare ground, and read
        // his own hearsay back off the map as confirmation - which
        // made every lie verify as true and left the whole
        // lie-detection apparatus unable to detect anything.
        //
        // This is the moment a lie is found out, and the only moment
        // it can be: the agent is standing on the spot and there is
        // nothing there. Sweeping a buffer of remembered claims every
        // hundred ticks caught almost none of them, because a claim
        // had to survive the buffer *and* the agent had to happen to
        // walk to it inside the same window.
        let centre = crate::world::Position::new(target_x, target_y);
        let found_out = agent.exploration_knowledge.hearsay_in_view(
            centre,
            exploration_radius,
            &really_here,
        );

        // What is not there when you are standing on it is not there
        agent
            .exploration_knowledge
            .known_resources
            .retain(|where_it_is, _| {
                let in_view = (where_it_is.x - centre.x).abs() <= exploration_radius
                    && (where_it_is.y - centre.y).abs() <= exploration_radius;
                !in_view || really_here.contains(where_it_is)
            });
        agent
            .exploration_knowledge
            .who_told_me
            .retain(|where_it_is, _| {
                let in_view = (where_it_is.x - centre.x).abs() <= exploration_radius
                    && (where_it_is.y - centre.y).abs() <= exploration_radius;
                !in_view || really_here.contains(where_it_is)
            });

        for (where_it_is, said, what_they_said) in found_out {
            if said.who == agent_id {
                continue;
            }
            let subject = format!("{:?}", what_they_said).to_lowercase();

            // Ground that has been worked looks worked, and somebody
            // else stripping a seam between the telling and the walk
            // is not evidence against the man who told you about it.
            if said.does_bare_ground_convict_him(
                self.current_tick,
                worked_out.contains(&where_it_is),
            ) {
                agent.found_out_i_was_lied_to(said.who, &subject, self.current_tick);
            } else {
                agent.found_out_they_were_out_of_date(said.who);
            }
        }

        // And what he was told was here, and is. Both copies of this
        // sweep only ever asked whether a claim had failed - see
        // ISSUES_FOUND #48 for why there are two of them - so a man's
        // standing could only ever fall.
        let borne_out = agent.exploration_knowledge.hearsay_borne_out(
            centre,
            exploration_radius,
            &really_here,
        );

        for (where_it_is, said) in borne_out {
            agent.exploration_knowledge.who_told_me.remove(&where_it_is);

            let how_much = self
                .world
                .resources
                .iter()
                .find(|resource| resource.position == where_it_is)
                .map(|resource| resource.amount)
                .unwrap_or(0);
            agent
                .exploration_knowledge
                .saw_it_again(where_it_is, how_much, self.current_tick);

            if said.who != agent_id {
                agent.found_out_they_were_right(said.who);
            }
        }

        // Discover nearby resources (within exploration radius)
        let mut discoveries = Vec::new();
        for resource in &self.world.resources {
            let resource_pos = crate::world::Position::new(
                resource.position.x,
                resource.position.y,
            );
            let dx = (resource_pos.x - target_x).abs();
            let dy = (resource_pos.y - target_y).abs();

            if dx <= exploration_radius && dy <= exploration_radius {
                if agent.exploration_knowledge.discover_resource(
                    resource_pos,
                    resource.resource_type,
                    self.current_tick,
                ) {
                    discoveries.push(format!("{:?}", resource.resource_type));
                }
            }
        }

        let _agent = &mut self.population.agents[agent_index];

        // Construct message about exploration results
        let mut message = format!(
            "Explored new area, discovered {} tiles",
            newly_explored_count
        );
        if !discoveries.is_empty() {
            message.push_str(&format!(", found: {}", discoveries.join(", ")));
        }

        debug!(
            "Agent {} explored to ({}, {}, {}), discovered {} new tiles",
            agent_id, target_x, target_y, target_z, newly_explored_count
        );

        // Grant Navigation XP for exploration (more for new discoveries)
        let agent = &mut self.population.agents[agent_index];
        let nav_xp = if newly_explored_count > 0 { 2 } else { 1 };
        agent.skills.practise(crate::agents::skills::SkillType::Navigation, nav_xp, tick_now);

        // Exploration is rewarding
        let curiosity_satisfaction = if newly_explored_count > 0 { 0.3 } else { 0.1 };

        ActionResult::success()
            .with_drive_change(DriveType::Curiosity, -curiosity_satisfaction)
            .with_energy_cost(5.0) // Exploration takes energy
            .with_message(message)
    }
}
