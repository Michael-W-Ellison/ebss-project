// src/analytics/happening/weather.rs
//! The weather, on a body.
//!
//! Exposure, the damage it does, and the two things a clear day is good for:
//! drying what was laid out, and noticing that it dried.
//!
//! Part of what happens whether or not anybody decides anything - see
//! [`super`]. Called from [`crate::analytics::turn`], in the order argued over
//! there.

use super::super::Simulation;
use log::{debug, warn};

impl Simulation {
    /// Whether the sky is doing anything that would dry a thing laid out in
    /// it.
    pub(in crate::analytics) fn is_the_sky_clear(&self) -> bool {
        matches!(
            self.world.climate.weather.weather_type,
            crate::environment::WeatherType::Clear
                | crate::environment::WeatherType::PartlyCloudy
        )
    }

    /// Whoever was standing near enough to see a thing dry out learns what
    /// dried it.
    ///
    /// The world does the drying; this is what turns it into something a
    /// person knows. It is the same shape as the four ways into farming: a
    /// thing happens, and whoever is near enough to see it happen takes the
    /// lesson. Nobody here is born knowing that cut flesh laid in the sun
    /// keeps and whole flesh laid in the sun does not - it has to be watched
    /// once.
    pub(in crate::analytics) fn who_saw_that_dry(&mut self) {
        let dried: Vec<(crate::world::Position, String)> =
            std::mem::take(&mut self.world.what_dried_in_the_sun);

        if dried.is_empty() {
            return;
        }

        for (where_it_is, what) in dried {
            for agent in self.population.agents.iter_mut() {
                if !agent.state.is_alive {
                    continue;
                }

                let paces = (agent.state.position.0 - where_it_is.x)
                    .abs()
                    .max((agent.state.position.1 - where_it_is.y).abs());

                if paces > Self::CLOSE_ENOUGH_TO_SEE_IT_COME_UP {
                    continue;
                }

                // This used to be where somebody found out that laying food
                // out keeps it. Everybody is born knowing that now - see
                // `Agent::what_anybody_is_born_knowing` - so what is left to
                // take from watching it is what it was worth: something that
                // would have been carrion is supper.
                debug!("Agent {} watched {what} dry out at {where_it_is:?}", agent.id);
                agent.lessons.record_particular("dry", true);
            }
        }
    }

    /// Process environmental damage for all agents
    pub fn process_environmental_damage(&mut self) {
        use crate::agents::body::{BodyPartType, InjuryType, CripplingType};
        use crate::world::{Position, TerrainType};
        use rand::Rng;
        let mut rng = crate::core::dice::roll();

        for agent in &mut self.population.agents {
            let agent_pos = Position::new(agent.state.position.0, agent.state.position.1);

            // Get terrain at agent position
            let terrain_type = self.world.grid.get_tile(&agent_pos)
                .map(|tile| tile.terrain.terrain_type)
                .unwrap_or(TerrainType::Plains);

            // Get actual temperature from climate system (returns f32 in Celsius)
            let temp_celsius = self.world.climate.get_temperature(agent_pos, terrain_type);

            // 1. EXPOSURE DAMAGE - Cold/Heat based on actual environment temperature
            let cold_insulation = agent.body.total_cold_insulation();
            let heat_resistance = agent.body.total_heat_resistance();

            // Cold exposure (temperature below 5°C with inadequate insulation)
            if temp_celsius < 5.0 {
                let cold_severity = ((5.0_f32 - temp_celsius) / 30.0).min(1.0); // Max severity at -25°C
                let effective_cold = cold_severity * (1.0 - cold_insulation.min(1.0));

                if effective_cold > 0.1 && rng.gen_bool((effective_cold * 0.02) as f64) {
                    let cold_damage = rng.gen_range(1.0..5.0) * effective_cold;
                    // Cold affects extremities most
                    let affected_parts = [
                        BodyPartType::LeftArm,
                        BodyPartType::RightArm,
                        BodyPartType::LeftLeg,
                        BodyPartType::RightLeg,
                    ];
                    let part = affected_parts[rng.gen_range(0..affected_parts.len())];

                    if let Some(body_part) = agent.body.get_part_mut(part) {
                        body_part.apply_injury(InjuryType::Minor, cold_damage, self.current_tick as u64);
                        debug!("Agent {} suffered cold exposure at {:.1}°C: {:.1} damage to {:?}",
                            agent.id, temp_celsius, cold_damage, part);
                    }
                }
            }

            // Heat exposure (temperature above 35°C with inadequate heat resistance)
            if temp_celsius > 35.0 {
                let heat_severity = ((temp_celsius - 35.0) / 20.0).min(1.0); // Max severity at 55°C
                let effective_heat = heat_severity * (1.0 - heat_resistance.min(1.0));

                if effective_heat > 0.1 && rng.gen_bool((effective_heat * 0.01) as f64) {
                    let heat_damage = rng.gen_range(2.0..8.0) * effective_heat;
                    // Heat affects torso and head
                    let affected_parts = [BodyPartType::Head, BodyPartType::Torso];
                    let part = affected_parts[rng.gen_range(0..affected_parts.len())];

                    if let Some(body_part) = agent.body.get_part_mut(part) {
                        body_part.apply_injury(InjuryType::Minor, heat_damage, self.current_tick as u64);
                        debug!("Agent {} suffered heat exposure at {:.1}°C: {:.1} damage to {:?}",
                            agent.id, temp_celsius, heat_damage, part);
                    }
                }
            }

            // 2. FALLING DAMAGE - Based on terrain type and elevation
            // Higher fall risk on mountains, hills, and near water (slippery)
            let fall_risk = match terrain_type {
                TerrainType::Mountain => 0.001,    // 0.1% - steep terrain
                TerrainType::Hills => 0.0003,      // 0.03% - uneven ground
                TerrainType::Riverbank => 0.0002,  // 0.02% - slippery banks
                TerrainType::Wetland => 0.0002,    // 0.02% - unstable footing
                TerrainType::Beach => 0.0001,      // 0.01% - uneven sand
                TerrainType::Forest => 0.00005,    // 0.005% - roots and obstacles
                _ => 0.00002,                      // 0.002% - flat terrain
            };

            if rng.gen_bool(fall_risk) {
                // Fall severity based on terrain
                let max_fall_height = match terrain_type {
                    TerrainType::Mountain => 5,
                    TerrainType::Hills => 3,
                    _ => 2,
                };
                let fall_height = rng.gen_range(1..=max_fall_height);
                let fall_damage = (fall_height as f32) * rng.gen_range(3.0..8.0);

                // Falls primarily affect legs, with chance of head/torso on severe falls
                let injured_part = if fall_height >= 4 && rng.gen_bool(0.3) {
                    if rng.gen_bool(0.5) { BodyPartType::Head } else { BodyPartType::Torso }
                } else {
                    if rng.gen_bool(0.5) { BodyPartType::LeftLeg } else { BodyPartType::RightLeg }
                };

                let injury_severity = if fall_damage >= 25.0 {
                    InjuryType::Crippling(CripplingType::Partial)
                } else if fall_damage >= 12.0 {
                    InjuryType::Major
                } else {
                    InjuryType::Minor
                };

                if let Some(body_part) = agent.body.get_part_mut(injured_part) {
                    body_part.apply_injury(injury_severity, fall_damage, self.current_tick as u64);
                    debug!("Agent {} fell on {:?} terrain: {:.1} damage to {:?} ({:?})",
                        agent.id, terrain_type, fall_damage, injured_part, injury_severity);
                }

                agent.state.lose_health(fall_damage * 0.15, "a fall");
            }

            // 3. DISEASE/INFECTION - Random chance
            // Agents with existing injuries have higher infection risk
            let injury_count: usize = agent.body.parts.values()
                .map(|part| part.injuries.len())
                .sum();

            if injury_count > 0 {
                let infection_chance = (injury_count as f64) * 0.0001; // 0.01% per injury per tick
                if rng.gen_bool(infection_chance) {
                    // Random body part gets infected
                    let parts: Vec<BodyPartType> = agent.body.parts.keys().cloned().collect();
                    if !parts.is_empty() {
                        let part = parts[rng.gen_range(0..parts.len())];
                        let _infection_damage = rng.gen_range(0.5..2.0);

                        if let Some(body_part) = agent.body.get_part_mut(part) {
                            body_part.add_condition(crate::agents::body::Condition {
                                condition_type: crate::agents::body::ConditionType::Infected,
                                severity: rng.gen_range(0.3..0.8),
                                duration: rng.gen_range(100..500), // Lasts 100-500 ticks
                            });
                            debug!("Agent {} developed infection on {:?}", agent.id, part);
                        }
                    }
                }
            }

            // 4. NATURAL HEALING - Process body tick (handles conditions, bleeding, etc.)
            agent.body.tick();
        }
    }

    pub(in crate::analytics) fn update_agent_exposure(&mut self) {
        let weather = self.world.climate.weather.clone();
        let time_of_day = self.world.climate.calendar.time_of_day;
        let now = self.current_tick;

        // Collect position data first to avoid borrow issues with climate.get_climate
        let agent_data: Vec<_> = self.population.agents.iter()
            .filter(|a| a.state.is_alive)
            .map(|a| {
                let pos = crate::world::Position::new(a.state.position.0, a.state.position.1);
                let terrain = self.world.grid.get_tile(&pos)
                    .map(|t| t.terrain.terrain_type)
                    .unwrap_or(crate::world::TerrainType::Plains);
                (a.id, pos, terrain)
            })
            .collect();

        // Get climate data for each agent position
        let climate_data: std::collections::BTreeMap<_, _> = agent_data.iter()
            .map(|(id, pos, terrain)| {
                let climate = self.world.climate.get_climate(*pos, *terrain);
                (*id, climate)
            })
            .collect();

        // Where the grown-ups are, so a baby can be counted as being held by
        // one of them
        let carers: Vec<(i32, i32, i32)> = self
            .population
            .agents
            .iter()
            .filter(|agent| agent.state.is_alive)
            .filter(|agent| {
                matches!(
                    agent.state.life_stage,
                    crate::agents::LifeStage::Adolescent
                        | crate::agents::LifeStage::Adult
                        | crate::agents::LifeStage::Elderly
                )
            })
            .map(|agent| agent.state.position)
            .collect();

        for agent in &mut self.population.agents {
            if !agent.state.is_alive {
                continue;
            }

            // Get the climate for this agent
            let climate = match climate_data.get(&agent.id) {
                Some(c) => c.clone(),
                None => continue,
            };

            // Get environmental temperature at agent's position
            let agent_pos = crate::world::Position::new(agent.state.position.0, agent.state.position.1);
            let terrain_type = self.world.grid.get_tile(&agent_pos)
                .map(|t| t.terrain.terrain_type)
                .unwrap_or(crate::world::TerrainType::Plains);

            let environmental_temp = climate.temperature;

            // Check if agent has shelter
            // Agent has shelter if they're in a completed building
            let mut has_shelter = self.world.buildings.iter().any(|b| {
                b.position == agent_pos && b.is_completed()
            }) || matches!(terrain_type, crate::world::TerrainType::Forest); // Forest provides partial shelter

            // The young are kept warm by whoever is looking after them.
            //
            // A child has no clothing of its own - it cannot gather flax, has
            // no skill to sew and nobody makes anything for it - so left to
            // the weather it runs two or three degrees colder than the adults
            // around it and dies of that. Nearly half of everyone ever born
            // died before growing up, which no birth rate can carry: it is
            // what emptied every settlement inside thirty thousand ticks.
            let too_young_to_manage = matches!(
                agent.state.life_stage,
                crate::agents::LifeStage::Infant | crate::agents::LifeStage::Child
            );

            if !has_shelter && too_young_to_manage {
                let position = agent.state.position;
                has_shelter = carers.iter().any(|carer| {
                    let dx = (carer.0 - position.0) as f32;
                    let dy = (carer.1 - position.1) as f32;
                    (dx * dx + dy * dy).sqrt()
                        <= crate::agents::childcare::MAX_CAREGIVER_DISTANCE
                });
            }

            // Update agent's body temperature based on climate, taking cover
            // into account so that reaching shelter actually warms the agent
            agent.update_temperature_with_shelter(&climate, has_shelter);

            // Check if agent has water access:
            // 1. Water containers in inventory (waterskin, water_flask, water_bucket)
            // 2. Near water terrain (river, lake)
            // 3. Near well building
            let has_water_container = agent.inventory.get_item("waterskin")
                .or_else(|| agent.inventory.get_item("water_flask"))
                .or_else(|| agent.inventory.get_item("water_bucket"))
                .map(|item| item.fill_percentage() > 0.1)
                .unwrap_or(false);

            let near_water_terrain = matches!(
                terrain_type,
                crate::world::TerrainType::Water |
                crate::world::TerrainType::Riverbank |
                crate::world::TerrainType::Wetland
            );

            let has_water_access = has_water_container || near_water_terrain;

            // Update exposure and apply damage
            let damage = agent.update_exposure(
                &weather,
                environmental_temp,
                has_shelter,
                has_water_access,
                time_of_day,
                now,
            );

            // Log critical exposure events
            if damage > 0.05 {
                debug!(
                    "Agent {} taking exposure damage: {:.3} (exposures: {:?})",
                    agent.id, damage, agent.exposure_status.active_exposures
                );
            }

            // Log body temperature issues
            if agent.body_temperature.is_hypothermic() {
                debug!(
                    "Agent {} is hypothermic! Body temp: {:.1}°C",
                    agent.id, agent.body_temperature.current
                );
            } else if agent.body_temperature.is_hyperthermic() {
                debug!(
                    "Agent {} is hyperthermic! Body temp: {:.1}°C",
                    agent.id, agent.body_temperature.current
                );
            }

            // If agent is in critical exposure condition, they may die
            if agent.exposure_status.is_critical() && agent.state.health < 20.0 {
                warn!(
                    "Agent {} in critical exposure condition! Health: {:.1}, Exposure: {:.2}",
                    agent.id, agent.state.health, agent.exposure_status.exposure_damage
                );
            }
        }
    }
}
