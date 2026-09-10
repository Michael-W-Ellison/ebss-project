// src/analytics/doing/ground.rs
//! Working the ground.
//!
//! Tilling, tending, muck, and moving a plant that is known to be good to
//! ground beside the camp.
//!
//! One method per `Action` variant, called from the dispatcher in
//! [`super::execute_action`]. The bodies are as they were when all fifty-two
//! lived in one five-thousand-line `match`; what changed is that a verb can
//! now be found, read and altered without scrolling past the other fifty-one.

use super::super::Simulation;
use crate::core::DriveType;
use crate::environment::ActionResult;
use log::debug;

impl Simulation {
    /// `Action::TillSoil`.
    pub(in crate::analytics) fn tilling_soil(&mut self, agent_index: usize, tick_now: u32) -> ActionResult {
        use crate::world::{Position, ResourceNode, TerrainType};

        let agent_position = self.population.agents[agent_index].state.position;
        let tile_position = Position::new(agent_position.0, agent_position.1);

        let ground = match self.world.grid.get_tile(&tile_position) {
            Some(tile) => tile.terrain.terrain_type,
            None => return ActionResult::failure("Nowhere to dig".to_string()),
        };

        // What is standing here, asked before whether the ground can be
        // broken - because the commonest green manure of all is a pod row in
        // a field somebody broke last year, and `can_be_tilled` says no to
        // ground that is already a field. Asking in the other order made
        // ploughing a crop in possible only on ground that had never been
        // farmed, which is the one place a farmer would not be doing it.
        let standing = self
            .world
            .resources
            .iter()
            .position(|resource| resource.position == tile_position);

        if let Some(standing) = standing {
            let crop = self.world.resources[standing].resource_type;
            let on_it = self.world.resources[standing].amount;

            // A pod row is worth more under the plough than in a basket when
            // the ground is poor, and turning it under is the whole point -
            // see `ploughing_a_crop_in`. Anything else standing here is
            // somebody's dinner and stays where it is.
            if crop.feeds_the_ground() && on_it > 0 {
                return self.ploughing_a_crop_in(agent_index, standing, tick_now);
            }

            return ActionResult::failure("Something already grows here".to_string());
        }

        if !crate::world::Terrain::new(ground).can_be_tilled() {
            return ActionResult::failure(format!(
                "Cannot break {:?} into a field",
                ground
            ));
        }

        if let Some(tile) = self.world.grid.get_tile_mut(&tile_position) {
            tile.terrain = crate::world::Terrain::new(TerrainType::Farmland);
        }

        // What goes in the ground is what the agent has to put in it,
        // and of what it has, whatever it has come to believe is worth
        // sowing. Nobody hands out grain seed: an agent that has only
        // ever stripped berry bushes sows berries, works the field all
        // season, and finds out what a berry bush thinks of a plough.
        // What this ground is worth, which is something a man standing on it
        // can see and which decides whether he puts a hungry crop in it.
        let how_good_the_ground_is = self
            .world
            .grid
            .get_tile(&tile_position)
            .map(|tile| tile.soil.fertility())
            .unwrap_or(0.5);

        let sown = Self::what_this_one_would_sow(
            &self.population.agents[agent_index],
            how_good_the_ground_is,
        );

        // The seed itself goes in the ground. Sowing was free before
        // this, which made a field a thing you got for a day's digging
        // rather than for a day's digging and a meal you did not eat.
        for (called, crop, _) in Self::what_can_be_sown() {
            if crop != sown {
                continue;
            }
            let agent = &mut self.population.agents[agent_index];
            if agent.how_many_i_have(called) > 0 {
                agent.inventory.remove_item(called, 1);
                break;
            }
        }

        // A newly sown field starts empty and fills as it grows
        let mut field = ResourceNode::new(
            sown,
            tile_position,
            Self::FIELD_YIELD,
        );
        field.amount = 0;
        self.world.resources.push(field);

        let agent = &mut self.population.agents[agent_index];
        agent
            .skills
            .practise(crate::agents::SkillType::Farming, 25, tick_now);

        debug!(
            "Agent {} broke ground at {:?} and sowed {:?}",
            agent.id, tile_position, sown
        );

        ActionResult::success()
            .with_drive_change(DriveType::Sustenance, -0.4)
            .with_energy_cost(12.0)
            .with_message("Broke ground and sowed a field".to_string())
    }

    /// Green manure: turning a standing crop under instead of eating it.
    ///
    /// Reached through `Action::TillSoil` when what is standing on the tile
    /// is a pod crop - see `ResourceType::feeds_the_ground`. Breaking ground
    /// that already carries something used to be refused flatly, which is
    /// right for a berry bush and wrong for a stand of vetch: ploughing the
    /// vetch in is not an obstacle to farming the tile, it is the oldest way
    /// there is of farming it.
    ///
    /// The trade is plain and it is a real one. What goes into the ground is
    /// the crop somebody could have carried home and eaten, and a hungry
    /// settlement will not do it. What comes back is the whole plant rather
    /// than the roots and stalk the growing pass already left, which is the
    /// part of the year's growth that would otherwise have walked away in a
    /// basket.
    pub(in crate::analytics) fn ploughing_a_crop_in(
        &mut self,
        agent_index: usize,
        standing: usize,
        tick_now: u32,
    ) -> ActionResult {
        use crate::world::{Soil, TerrainType};

        let where_it_stands = self.world.resources[standing].position;
        let turned_under = self.world.resources[standing].amount;
        let crop = self.world.resources[standing].resource_type;

        self.world.resources.remove(standing);

        if let Some(tile) = self.world.grid.get_tile_mut(&where_it_stands) {
            // The harvestable part, which is exactly what the residue path
            // does *not* leave behind: `regenerate_in_ground` gives the
            // ground the roots and the stalk of everything that grows, and
            // the rest is what somebody carries off. Turning the crop under
            // is choosing not to carry it off.
            tile.soil.feed(turned_under as f32 * Soil::NUTRIENT_PER_UNIT_GROWN);
            tile.soil
                .add_leaf_litter(turned_under as f32 * Soil::RESIDUE_PER_UNIT_GROWN);

            // And the ground is broken now, which is the other half of what
            // a day behind a plough buys.
            if crate::world::Terrain::new(tile.terrain.terrain_type).can_be_tilled() {
                tile.terrain = crate::world::Terrain::new(TerrainType::Farmland);
            }
            tile.soil.somebody_worked_this_field();
        }

        let agent = &mut self.population.agents[agent_index];
        agent
            .skills
            .practise(crate::agents::SkillType::Farming, 15, tick_now);

        debug!(
            "Agent {} turned {turned_under} of {crop:?} under at {where_it_stands:?}",
            agent.id
        );

        ActionResult::success()
            .with_energy_cost(10.0)
            .with_message(format!("Turned {turned_under} of {crop:?} under"))
    }

    /// `Action::TakeCutting`.
    pub(in crate::analytics) fn taking_a_cutting(&mut self, agent_index: usize, tick_now: u32) -> ActionResult {
        use crate::agents::InventoryItem;
        use crate::world::Position;

        let agent_position = self.population.agents[agent_index].state.position;
        let here = Position::new(agent_position.0, agent_position.1);

        let Some(index) = self.world.resources.iter().position(|resource| {
            resource.position == here
                && resource.amount > Self::WHAT_A_CUTTING_TAKES
                && Self::what_can_be_sown()
                    .into_iter()
                    .any(|(_, sowable, _)| sowable == resource.resource_type)
        }) else {
            return ActionResult::failure("Nothing here worth lifting".to_string());
        };

        let crop = self.world.resources[index].resource_type;

        let Some((called, _, _)) = Self::what_can_be_sown()
            .into_iter()
            .find(|(_, sowable, _)| *sowable == crop)
        else {
            return ActionResult::failure(format!("{crop:?} does not move"));
        };

        if self.world.resources[index].max_amount
            <= Self::TOO_THIN_TO_DIG + Self::WHAT_A_CUTTING_TAKES
        {
            return ActionResult::failure("Too thin to dig out of".to_string());
        }

        // Taking a cutting costs the plant it came off, permanently: a
        // slip is a piece of the plant and not a piece of this year's
        // crop. A patch dug over for slips carries less from now on.
        self.world.resources[index].harvest(Self::WHAT_A_CUTTING_TAKES);
        self.world.resources[index].max_amount = self.world.resources[index]
            .max_amount
            .saturating_sub(Self::WHAT_A_CUTTING_TAKES);

        let agent = &mut self.population.agents[agent_index];
        agent.inventory.add_item(InventoryItem::new_with_weight(
            Self::a_cutting_of(called),
            1,
            1.5,
        ));
        agent
            .skills
            .practise(crate::agents::SkillType::Farming, 8, tick_now);

        debug!("Agent {} lifted a slip of {called} at {here:?}", agent.id);

        ActionResult::success()
            .with_energy_cost(5.0)
            .with_message(format!("Lifted a slip of {called}"))
    }

    /// `Action::PlantCutting`.
    pub(in crate::analytics) fn planting_a_cutting(&mut self, agent_index: usize, tick_now: u32) -> ActionResult {
        use crate::world::{Position, ResourceNode};

        let agent_position = self.population.agents[agent_index].state.position;
        let here = Position::new(agent_position.0, agent_position.1);

        let Some((called, crop)) =
            Self::a_cutting_in_the_pack(&self.population.agents[agent_index])
        else {
            return ActionResult::failure("Nothing to plant".to_string());
        };

        let will_take = self
            .world
            .grid
            .get_tile(&here)
            .map(|tile| tile.terrain.can_be_tilled() || tile.terrain.is_cultivated())
            .unwrap_or(false);

        if !will_take {
            return ActionResult::failure("Nothing will take here".to_string());
        }

        if self
            .world
            .resources
            .iter()
            .any(|resource| resource.position == here)
        {
            return ActionResult::failure("Something already grows here".to_string());
        }

        let mut moved = ResourceNode::new(crop, here, Self::WHAT_A_MOVED_PLANT_COMES_TO);
        moved.amount = Self::WHAT_A_CUTTING_STARTS_WITH;
        self.world.resources.push(moved);

        let agent = &mut self.population.agents[agent_index];
        agent
            .inventory
            .remove_item(&Self::a_cutting_of(called), 1);
        agent
            .skills
            .practise(crate::agents::SkillType::Farming, 15, tick_now);

        debug!("Agent {} put a slip of {called} in at {here:?}", agent.id);

        ActionResult::success()
            .with_drive_change(DriveType::Sustenance, -0.3)
            .with_energy_cost(8.0)
            .with_message(format!("Put a slip of {called} in beside the camp"))
    }

    /// `Action::TendField`.
    pub(in crate::analytics) fn tending_a_field(&mut self, agent_index: usize, tick_now: u32) -> ActionResult {
        use crate::world::Position;

        let agent_position = self.population.agents[agent_index].state.position;
        let tile_position = Position::new(agent_position.0, agent_position.1);

        let Some(tile) = self.world.grid.get_tile(&tile_position) else {
            return ActionResult::failure("Nowhere to work".to_string());
        };

        if !tile.terrain.is_cultivated() {
            return ActionResult::failure("No field here to work".to_string());
        }

        let before = tile.soil.weeds + tile.soil.pests;

        if before <= 0.0 {
            return ActionResult::failure("Nothing wants doing here".to_string());
        }

        // A walk out to a field that is still bare after all this work
        // is what teaches an agent that it sowed the wrong thing.
        let standing = self
            .world
            .resources
            .iter()
            .find(|resource| resource.position == tile_position)
            .map(|resource| (resource.resource_type, resource.amount));

        if let Some((crop, amount)) = standing {
            if let Some((called, _, _)) = Self::what_can_be_sown()
                .into_iter()
                .find(|(_, sowable, _)| *sowable == crop)
            {
                self.population.agents[agent_index]
                    .lessons
                    .record_particular(&format!("sow:{called}"), amount > 0);
            }

            // And whether the whole business is worth anybody's day.
            // A man standing in his own field can see whether there is
            // anything in it; he does not have to wait until he is
            // carrying it home. This is where farming is mostly either
            // confirmed or given up on.
            self.population.agents[agent_index]
                .practices
                .record_outcome(crate::agents::practices::Practice::Farming, amount > 0);
        }

        // What a practised hand gets through in a turn. Somebody who
        // has done it for years knows a weed from a seedling and takes
        // the whole field; somebody who has not clears half of it and
        // treads on the rest.
        let hand = self.population.agents[agent_index]
            .skills
            .hand_for(crate::agents::SkillType::Farming);

        let cleared = {
            let Some(tile) = self.world.grid.get_tile_mut(&tile_position) else {
                return ActionResult::failure("Nowhere to work".to_string());
            };

            for _ in 0..(hand.round() as u32).clamp(1, 3) {
                tile.soil.somebody_worked_this_field();
            }

            before - (tile.soil.weeds + tile.soil.pests)
        };

        let agent = &mut self.population.agents[agent_index];
        agent
            .skills
            .practise(crate::agents::SkillType::Farming, 12, tick_now);

        debug!(
            "Agent {} worked the field at {:?} (weeds and pests down {:.2})",
            agent.id, tile_position, cleared
        );

        ActionResult::success()
            .with_drive_change(DriveType::Sustenance, -0.2)
            .with_energy_cost(7.0)
            .with_message(format!("Worked the field, clearing {cleared:.2}"))
    }

    /// `Action::SpreadMuck`.
    pub(in crate::analytics) fn spreading_muck(&mut self, agent_index: usize, tick_now: u32) -> ActionResult {
        use crate::agents::practices::Practice;
        use crate::world::Position;

        let agent_position = self.population.agents[agent_index].state.position;
        let tile_position = Position::new(agent_position.0, agent_position.1);

        // What is in the pack that is fit for nothing else
        let refuse: Vec<(String, u32)> = {
            let agent = &self.population.agents[agent_index];
            agent
                .inventory
                .get_all_items()
                .values()
                .filter(|item| item.quantity > 0)
                .filter(|item| {
                    item.food_data
                        .as_ref()
                        .map(|food| food.is_rotting() || food.is_ruined())
                        .unwrap_or(false)
                })
                .map(|item| (item.item_id.clone(), item.quantity))
                .collect()
        };

        if refuse.is_empty() {
            return ActionResult::failure("Nothing spoiled to tip out".to_string());
        }

        let before = self
            .world
            .grid
            .get_tile(&tile_position)
            .map(|tile| tile.soil.fertility() + tile.soil.litter())
            .unwrap_or(0.0);

        let mut tipped = 0;
        let mut worth = 0.0;
        {
            let agent = &mut self.population.agents[agent_index];
            for (item_id, quantity) in &refuse {
                agent.inventory.remove_item(item_id, *quantity);
                tipped += quantity;

                // A rotten fish is not a rotten turnip. The turnip is
                // giving back what this ground grew it with; the fish
                // is bringing in what the sea grew it with, and is
                // worth many times as much to a field on that account
                // alone.
                worth += *quantity as f32
                    * if crate::world::Soil::came_out_of_the_water(item_id) {
                        Self::MUCK_PER_FISH
                    } else {
                        Self::MUCK_PER_UNIT
                    };
            }
        }

        // Spoiled food is soft matter and goes quickly, given wet ground
        if let Some(tile) = self.world.grid.get_tile_mut(&tile_position) {
            tile.soil.add_leaf_litter(worth);
        }

        let after = self
            .world
            .grid
            .get_tile(&tile_position)
            .map(|tile| tile.soil.fertility() + tile.soil.litter())
            .unwrap_or(0.0);

        // What the agent can actually see: the ground here is richer
        // than it was. Whether that was worth doing is a judgement it
        // makes for itself, and gets wrong sometimes - tipping muck on
        // bare rock or in a desert does nothing much.
        let worked = after > before + 0.05;

        let agent = &mut self.population.agents[agent_index];
        agent.practices.record_outcome(Practice::SpreadingMuck, worked);
        agent
            .skills
            .practise(crate::agents::SkillType::Farming, 10, tick_now);

        debug!(
            "Agent {} tipped {} spoiled units onto {:?} (ground {:.2} -> {:.2})",
            agent.id, tipped, tile_position, before, after
        );

        ActionResult::success()
            .with_energy_cost(3.0)
            .with_message(format!("Spread {} of muck on the ground", tipped))
    }
}
