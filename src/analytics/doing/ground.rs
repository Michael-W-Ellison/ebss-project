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
    /// Somebody standing over the crop at `resource_index` sees how heavy it
    /// stands, and makes what he will of the field under it.
    ///
    /// The one place a look at a crop becomes a belief about a field, so that
    /// every hand that takes a crop - gathering it, eating it off the stalk,
    /// weeding round it - reads it the same way. See `Agent::saw_a_stand_on`
    /// for what a farmer can and cannot tell from it.
    ///
    /// What a stand says is the grade that would carry a stand that heavy as
    /// a full crop on broken ground. A ripe crop is the one to read: it has
    /// finished growing, so what it ripened at is what the ground carried,
    /// and since nobody picks a field crop until it is ripe, everybody who
    /// takes a harvest reads one. A crop still growing, seen while weeding,
    /// can only say the ground is at least that good. Wild ground is not read
    /// here: nothing anybody does to it changes it, so there is nothing to
    /// find out.
    pub(in crate::analytics) fn looking_at_the_crop(&mut self, agent_index: usize, resource_index: usize) {
        use crate::world::SoilGrade;

        let Some(crop) = self.world.resources.get(resource_index) else {
            return;
        };
        let at = crop.position;
        if self.world.grid.field_at(&at).is_none() {
            return;
        }

        let a_full_stand = crop.how_heavy_a_crop_it_carries(
            self.world.grid.what_it_yields_here_for(&at, crop.resource_type),
        );
        let on_ordinary_loam =
            crop.how_heavy_a_crop_it_carries(crop.resource_type.what_the_plough_does_for_it());
        if on_ordinary_loam == 0 {
            return;
        }

        // Ripe, it is read at what it ripened at, whatever has come off it
        // since; growing, at what stands.
        let stand = if crop.ripe_stand > 0 { crop.ripe_stand } else { crop.amount };
        let it_says = SoilGrade::nearest_to(stand as f32 / on_ordinary_loam as f32);
        let full = a_full_stand > 0 && stand >= a_full_stand;

        self.population.agents[agent_index].saw_a_stand_on((at.x, at.y), it_says, full);
    }

    /// `Action::ScatterSeed`: a handful of gathered seed thrown on good
    /// ground where the agent stands.
    ///
    /// Stage 1 of the agricultural path. Nothing is broken and nothing is
    /// tended; the seed comes up, or does not, as a stand like any other, and
    /// it remembers who threw it (`ResourceNode::sown_by`), because gathering
    /// from your own stand is what teaches that this works.
    pub(in crate::analytics) fn scattering_seed(&mut self, agent_index: usize) -> ActionResult {
        use crate::world::{Position, ResourceNode};

        let at = self.population.agents[agent_index].state.position;
        let here = Position::new(at.0, at.1);

        let takes_seed = self
            .world
            .grid
            .get_tile(&here)
            .map(|tile| tile.terrain.can_be_tilled() || tile.terrain.is_cultivated())
            .unwrap_or(false);
        if !takes_seed {
            return ActionResult::failure("Nothing would come up here".to_string());
        }
        if self.world.nodes_on(here).next().is_some() {
            return ActionResult::failure("Something already grows here".to_string());
        }

        let Some((seed, crop)) = Self::seed_worth_scattering(&self.population.agents[agent_index]) else {
            return ActionResult::failure("No seed to scatter".to_string());
        };

        let sower = {
            let agent = &mut self.population.agents[agent_index];
            agent.inventory.remove_item(seed, 1);
            agent.skills.made_one(crate::agents::Agent::SEED_SCATTERED);
            agent.id
        };

        let mut stand = ResourceNode::new(crop, here, Self::WHAT_ONE_SEED_COMES_TO);
        stand.amount = 1;
        stand.sown_by = Some(sower);
        stand.sown_on = Some(self.world.turn);
        if seed == crate::agents::Agent::SEED {
            stand.bred = self.population.agents[agent_index].seed_line;
        }
        self.world.put_a_node_down(stand);

        ActionResult::success()
            .with_energy_cost(3.0)
            .with_message(format!("Scattered {seed} at {here:?}"))
    }

    /// The seed in a pack worth throwing on the ground: something that feeds
    /// somebody, the first of `what_can_be_sown` that is carried.
    pub(in crate::analytics) fn seed_worth_scattering(
        agent: &crate::agents::Agent,
    ) -> Option<(&'static str, crate::world::ResourceType)> {
        Self::what_can_be_sown()
            .into_iter()
            .filter(|(_, _, feeds_anybody)| *feeds_anybody)
            .find(|(called, _, _)| agent.how_many_i_have(called) > 0)
            .map(|(called, crop, _)| (called, crop))
    }

    /// `Action::TillSoil`.
    pub(in crate::analytics) fn tilling_soil(&mut self, agent_index: usize, turn_now: u32) -> ActionResult {
        use crate::world::{Position, ResourceNode};

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
            .node_numbers_on(tile_position)
            .first()
            .copied();

        if let Some(standing) = standing {
            let crop = self.world.resources[standing].resource_type;
            let on_it = self.world.resources[standing].amount;

            // A pod row is worth more under the plough than in a basket when
            // the ground is poor, and turning it under is the whole point -
            // see `ploughing_a_crop_in`.
            if crop.feeds_the_ground() && on_it > 0 {
                return self.ploughing_a_crop_in(agent_index, standing, turn_now);
            }

            // And so is whatever stands on a field its farmer thinks is worn
            // to nothing: what comes off it is not worth the picking, and the
            // ground wants something else in it. This is how a field changes
            // crop - it goes under, and is sown again - and without it a field
            // carried the crop it was first sown with for ever.
            if self.population.agents[agent_index].what_i_make_of_the_field_at((tile_position.x, tile_position.y))
                == Some(crate::world::SoilGrade::LADDER[0])
            {
                return self.ploughing_a_crop_in(agent_index, standing, turn_now);
            }

            return ActionResult::failure("Something already grows here".to_string());
        }

        // Open grass is broken; a field with nothing standing on it is sown
        // again. A bare field used to be refused as though it were not ground
        // at all, so a crop ploughed in left the field bare for ever.
        let ground_will_take_seed = crate::world::Terrain::new(ground).can_be_tilled()
            || crate::world::Terrain::new(ground).is_cultivated();
        if !ground_will_take_seed {
            return ActionResult::failure(format!(
                "Cannot break {:?} into a field",
                ground
            ));
        }

        // Nothing broken for a crop out of its time, by somebody who keeps a
        // planting calendar (#418). Asked before the ground is broken, since
        // a calendar says nothing about the ground.
        let today = self.world.climate.calendar.day_of_year;
        if Self::what_this_one_would_sow_on(&self.population.agents[agent_index], 1.0, Some(today)).is_none() {
            return ActionResult::failure("Nothing to sow that is in its time".to_string());
        }

        // Broken, and on the record as a field from today: what kind of ground
        // it is and what grade it was, which a rest brings it back to.
        let now = self.world.turn;
        self.world.grid.break_ground(&tile_position, now);

        // What goes in the ground is what the agent has to put in it,
        // and of what it has, whatever it has come to believe is worth
        // sowing. Nobody hands out grain seed: an agent that has only
        // ever stripped berry bushes sows berries, works the field all
        // season, and finds out what a berry bush thinks of a plough.
        // What this ground is worth, which decides whether he puts a hungry
        // crop in it. Wild ground he can see: what grows on it wild is what it
        // is, and nobody has ever changed it. A field is what he makes of it
        // from what has come off it - see `Agent::saw_a_stand_on` - and a
        // field he has never seen a crop on he takes for ordinary ground.
        let how_good_the_ground_is = if self.world.grid.field_at(&tile_position).is_some() {
            crate::world::resources::ResourceNode::WHAT_ORDINARY_WILD_GROUND_CARRIES
                * self.population.agents[agent_index]
                    .what_i_make_of_the_field_at((tile_position.x, tile_position.y))
                    .unwrap_or(crate::world::SoilGrade::Ordinary)
                    .multiplier()
        } else {
            self.world.grid.how_good_the_ground_is(&tile_position)
        };

        // By a planting calendar, only what is in its time (#418)
        let Some(sown) = Self::what_this_one_would_sow_on(
            &self.population.agents[agent_index],
            how_good_the_ground_is,
            Some(today),
        ) else {
            return ActionResult::failure("Nothing to sow that is in its time".to_string());
        };

        // A field of the crop somebody has settled on (#421)
        {
            let agent = &self.population.agents[agent_index];
            let staple = agent.knows_the_way("staple").then(|| agent.my_staple_crop()).flatten();
            if staple.is_some_and(|called| {
                Self::what_can_be_sown().into_iter().any(|(name, crop, _)| name == called && crop == sown)
            }) {
                self.world.staple_fields_sown += 1;
            }
        }

        // The seed itself goes in the ground. Sowing was free before
        // this, which made a field a thing you got for a day's digging
        // rather than for a day's digging and a meal you did not eat.
        let mut sown_of = None;
        for (called, crop, _) in Self::what_can_be_sown() {
            if crop != sown {
                continue;
            }
            let agent = &mut self.population.agents[agent_index];
            if agent.how_many_i_have(called) > 0 {
                agent.inventory.remove_item(called, 1);
                sown_of = Some(called);
                break;
            }
        }

        // Ground broken with no seed kept back for it, often enough, is how
        // anybody thinks to keep the next year's seed back off the harvest
        // (#396)
        {
            let agent = &mut self.population.agents[agent_index];
            if sown_of != Some(crate::agents::Agent::SEED) {
                agent.skills.made_one(crate::agents::Agent::SOWN_WITHOUT_SEED);
                if agent.skills.times_made(crate::agents::Agent::SOWN_WITHOUT_SEED)
                    >= crate::agents::Agent::TIMES_SOWN_WITHOUT_SEED_BEFORE_SAVING
                {
                    agent.it_comes_to_me(
                        "know:save_seed",
                        crate::agents::Agent::TIMES_SOWN_WITHOUT_SEED_BEFORE_SAVING,
                    );
                }
            }
        }

        // A newly sown field starts empty and fills as it grows: of seed bred
        // as far as the sower's has been, if it was kept seed (#397)
        let mut field = ResourceNode::new(
            sown,
            tile_position,
            Self::FIELD_YIELD,
        );
        field.amount = 0;
        field.sown_on = Some(now);
        // And how much went in: by the measure, or at a guess (#409)
        if let Some(called) = sown_of {
            if self.sown_at_a_guess(agent_index, called) {
                field.sown_thin_by = Some(self.population.agents[agent_index].id);
            }
        }
        if sown_of == Some(crate::agents::Agent::SEED) {
            field.bred = self.population.agents[agent_index].seed_line;
        }
        // What this crop has cost already: the turn of hard work, and the
        // handful that went in the ground instead of into the sower (#416)
        let seed_cost = if sown_of.is_some() { self.what_an_item_of_this_crop_is_worth(sown) } else { 0.0 };
        field.somebody_put_into_the_crop(
            self.population.agents[agent_index].id,
            Self::what_a_turn_of_this_work_costs(Self::BREAKING_GROUND_COSTS) + seed_cost,
        );
        self.world.put_a_node_down(field);

        let agent = &mut self.population.agents[agent_index];
        agent
            .skills
            .practise(crate::agents::SkillType::Farming, 25, turn_now);
        // Breaking ground often enough is how a hoe comes to mind (#385)
        agent.skills.made_one(crate::environment::making::GROUND_BROKEN_FOR_A_GARDEN);
        if agent.skills.times_made(crate::environment::making::GROUND_BROKEN_FOR_A_GARDEN)
            >= crate::environment::making::TIMES_GROUND_IS_BROKEN_BEFORE_A_HOE
            && agent.it_comes_to_me("hoe", crate::environment::making::TIMES_GROUND_IS_BROKEN_BEFORE_A_HOE)
        {
            debug!("Agent {} thought of a blade set across a haft", agent.id);
        }

        debug!(
            "Agent {} broke ground at {:?} and sowed {:?}",
            agent.id, tile_position, sown
        );

        ActionResult::success()
            .with_drive_change(DriveType::Sustenance, -0.4)
            .with_energy_cost(Self::BREAKING_GROUND_COSTS)
            .with_message("Broke ground and sowed a field".to_string())
    }

    /// What a turn breaking ground and sowing it costs, and a turn weeding.
    pub(in crate::analytics) const BREAKING_GROUND_COSTS: f32 = 12.0;
    pub(in crate::analytics) const WEEDING_COSTS: f32 = 7.0;

    /// What a turn of work at this cost takes out of a body, in the energy
    /// food is reckoned in (#416): a turn's worth of minutes, burned at the
    /// rate the work burns.
    pub(in crate::analytics) fn what_a_turn_of_this_work_costs(energy_cost: f32) -> f32 {
        crate::environment::seasons::MINUTES_PER_TURN as f32
            * crate::agents::physiology::what_the_work_costs(energy_cost)
    }

    /// What one handful of this crop is worth to eat, in energy (#416).
    pub(in crate::analytics) fn what_an_item_of_this_crop_is_worth(&self, crop: crate::world::ResourceType) -> f32 {
        use crate::agents::physiology::{what_a_unit_of_this_is_worth, ENERGY_OF_ORDINARY_FOOD, UNITS_IN_ONE_ITEM};
        let energy = Self::edible_item_for(crop)
            .and_then(|item| self.food_database.get(&item))
            .map(|template| template.base_nutrition.energy)
            .unwrap_or(ENERGY_OF_ORDINARY_FOOD);
        UNITS_IN_ONE_ITEM * what_a_unit_of_this_is_worth(energy)
    }

    /// How often seed thrown by the eye goes in too thin for the ground, and
    /// how often too thick, a second handful wasted on it (#409): a third
    /// each, and right the rest of the time.
    pub(in crate::analytics) const A_GUESS_SOWS_THIN: f64 = 1.0 / 3.0;
    pub(in crate::analytics) const A_GUESS_SOWS_THICK: f64 = 1.0 / 3.0;

    /// Whether this one sows by the measure, knowing it: anybody but the
    /// lazy, who throw it and have done (#409).
    pub(in crate::analytics) fn would_sow_by_the_measure(agent: &crate::agents::Agent) -> bool {
        agent.knows_the_way("measured_sowing") && !agent.traits.has(crate::core::traits::Trait::Lazy)
    }

    /// The seed is in the ground: how much of it, and whether the field it
    /// went into is sown thin (#409). Somebody who sows by the measure puts in
    /// what the ground takes. Anybody else throws it by the eye: a third of
    /// the time too little, and the field carries less, and a third too much,
    /// and a second handful of seed, if there is one, goes in for nothing.
    pub(in crate::analytics) fn sown_at_a_guess(&mut self, agent_index: usize, called: &str) -> bool {
        use rand::Rng;

        if Self::would_sow_by_the_measure(&self.population.agents[agent_index]) {
            self.world.fields_sown_measured += 1;
            return false;
        }
        let guess: f64 = crate::core::dice::roll().gen();
        if guess < Self::A_GUESS_SOWS_THIN {
            self.world.fields_sown_thin += 1;
            return true;
        }
        if guess < Self::A_GUESS_SOWS_THIN + Self::A_GUESS_SOWS_THICK {
            let agent = &mut self.population.agents[agent_index];
            if agent.how_many_i_have(called) > 0 {
                agent.inventory.remove_item(called, 1);
                self.world.fields_sown_thick += 1;
            }
        }
        false
    }

    /// What the practice record calls a harvest off a field one sowed thin,
    /// and how many before somebody thinks to sow by the measure (#409).
    pub(in crate::analytics) const A_THIN_FIELD_GATHERED: &'static str = "thinfieldgathered";
    pub(in crate::analytics) const THIN_FIELDS_BEFORE_A_MEASURE: u32 = 3;

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
        turn_now: u32,
    ) -> ActionResult {
        let where_it_stands = self.world.resources[standing].position;
        let turned_under = self.world.resources[standing].amount;
        let crop = self.world.resources[standing].resource_type;

        self.world.take_a_node_up(standing);

        // The ground is broken now, which is half of what a day behind a
        // plough buys. The other half is the crop itself going in as muck: a
        // green manure is manure, and comes to a rung a season after it goes
        // under, the same as anything carted onto a field - see
        // `Field::somebody_mucked_it`.
        let now = self.world.turn;
        self.world.grid.break_ground(&where_it_stands, now);
        self.world.grid.somebody_mucked(
            &where_it_stands,
            turned_under as f32 * Self::MUCK_PER_UNIT,
            now,
        );
        if let Some(tile) = self.world.grid.get_tile_mut(&where_it_stands) {
            tile.soil.somebody_worked_this_field();
        }

        let agent = &mut self.population.agents[agent_index];
        agent
            .skills
            .practise(crate::agents::SkillType::Farming, 15, turn_now);

        debug!(
            "Agent {} turned {turned_under} of {crop:?} under at {where_it_stands:?}",
            agent.id
        );

        ActionResult::success()
            .with_energy_cost(10.0)
            .with_message(format!("Turned {turned_under} of {crop:?} under"))
    }

    /// `Action::TakeCutting`.
    pub(in crate::analytics) fn taking_a_cutting(&mut self, agent_index: usize, turn_now: u32) -> ActionResult {
        use crate::agents::InventoryItem;
        use crate::world::Position;

        let agent_position = self.population.agents[agent_index].state.position;
        let here = Position::new(agent_position.0, agent_position.1);

        let Some(index) = self.world.node_numbers_on(here).into_iter().find(|&number| {
            let resource = &self.world.resources[number];
            resource.amount > Self::WHAT_A_CUTTING_TAKES
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
        self.world.did_that_empty_it(index);
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
            .practise(crate::agents::SkillType::Farming, 8, turn_now);

        debug!("Agent {} lifted a slip of {called} at {here:?}", agent.id);

        ActionResult::success()
            .with_energy_cost(5.0)
            .with_message(format!("Lifted a slip of {called}"))
    }

    /// `Action::PlantCutting`.
    pub(in crate::analytics) fn planting_a_cutting(&mut self, agent_index: usize, turn_now: u32) -> ActionResult {
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

        if self.world.nodes_on(here).next().is_some() {
            return ActionResult::failure("Something already grows here".to_string());
        }

        let mut moved = ResourceNode::new(crop, here, Self::WHAT_A_MOVED_PLANT_COMES_TO);
        moved.amount = Self::WHAT_A_CUTTING_STARTS_WITH;
        self.world.put_a_node_down(moved);

        let agent = &mut self.population.agents[agent_index];
        agent
            .inventory
            .remove_item(&Self::a_cutting_of(called), 1);
        agent
            .skills
            .practise(crate::agents::SkillType::Farming, 15, turn_now);

        debug!("Agent {} put a slip of {called} in at {here:?}", agent.id);

        ActionResult::success()
            .with_drive_change(DriveType::Sustenance, -0.3)
            .with_energy_cost(8.0)
            .with_message(format!("Put a slip of {called} in beside the camp"))
    }

    /// `Action::TendField`.
    pub(in crate::analytics) fn tending_a_field(&mut self, agent_index: usize, turn_now: u32) -> ActionResult {
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

        // What share of it is pests, on a field a hedge keeps the beasts off
        // or not: weeding both is how anybody sees what a hedge does (#398)
        let pests_share = f64::from(tile.soil.pests / before);
        let hedged = {
            let now = self.world.turn;
            self.world
                .thorn_hedges
                .iter()
                .any(|hedge| hedge.is_closed(now) && hedge.takes_in((tile_position.x, tile_position.y)))
        };
        {
            let agent = &mut self.population.agents[agent_index];
            agent.what_i_saw_of_a_field(hedged, pests_share);
            if agent.what_the_fields_have_shown_me() {
                debug!("Agent {} saw the hedged fields fare better", agent.id);
            }
        }

        // A walk out to a field that is still bare after all this work
        // is what teaches an agent that it sowed the wrong thing - and a walk
        // round a crop is a look at how heavy it stands.
        let standing_at = self
            .world
            .node_numbers_on(tile_position)
            .first()
            .copied();
        if let Some(index) = standing_at {
            self.looking_at_the_crop(agent_index, index);
            // And a turn's work into the crop (#416)
            let who = self.population.agents[agent_index].id;
            self.world.resources[index]
                .somebody_put_into_the_crop(who, Self::what_a_turn_of_this_work_costs(Self::WEEDING_COSTS));
        }
        let standing = standing_at.map(|index| {
            let resource = &self.world.resources[index];
            (resource.resource_type, resource.amount)
        });

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
        // And what it is done with: a stick pries a weed out, a hoe chops a
        // row of them (#385). Only ever more: bare hands weed as they did.
        let tool = self.population.agents[agent_index]
            .how_much_my_tools_bring_back(crate::agents::SkillType::Farming)
            .max(1.0);
        let hand = hand * tool;

        let cleared = {
            let Some(tile) = self.world.grid.get_tile_mut(&tile_position) else {
                return ActionResult::failure("Nowhere to work".to_string());
            };

            for _ in 0..(hand.round() as u32).clamp(1, 3) {
                tile.soil.somebody_worked_this_field();
            }

            before - (tile.soil.weeds + tile.soil.pests)
        };

        // A field somebody is keeping is not a field anybody has given up on.
        let now = self.world.turn;
        self.world.grid.somebody_worked_the_field(&tile_position, now);

        let agent = &mut self.population.agents[agent_index];
        agent
            .skills
            .practise(crate::agents::SkillType::Farming, 12, turn_now);

        debug!(
            "Agent {} worked the field at {:?} (weeds and pests down {:.2})",
            agent.id, tile_position, cleared
        );

        ActionResult::success()
            .with_drive_change(DriveType::Sustenance, -0.2)
            .with_energy_cost(Self::WEEDING_COSTS)
            .with_message(format!("Worked the field, clearing {cleared:.2}"))
    }

    /// `Action::SpreadMuck`.
    pub(in crate::analytics) fn spreading_muck(&mut self, agent_index: usize, turn_now: u32) -> ActionResult {
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
                    item.item_id == "dung"
                        || item.item_id == Self::COMPOST
                        || item
                            .food_data
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
                    * if item_id == "dung" || item_id == Self::COMPOST {
                        // What a beast has already made of the grass (#354),
                        // or the midden of the camp's refuse (#386)
                        2.0 * Self::MUCK_PER_UNIT
                    } else if crate::world::Soil::came_out_of_the_water(item_id) {
                        Self::MUCK_PER_FISH
                    } else {
                        Self::MUCK_PER_UNIT
                    };
            }
        }

        // Onto a field it goes into the ground, and a season on it comes to a
        // rung - see `Field::somebody_mucked_it`. Anywhere else it is only
        // muck on the ground: wild ground is what it was made.
        //
        // Whether that was worth doing is a judgement the agent makes for
        // itself: muck tipped on a field that can still take it is muck that
        // will tell, and muck tipped anywhere else never will.
        let now = self.world.turn;
        let worn = self.world.grid.field_at(&tile_position).is_some_and(|field| field.grade < field.natural);
        let worked = self.world.grid.somebody_mucked(&tile_position, worth, now);
        // Carried to a worn field on purpose (#419)
        if worked && worn && self.population.agents[agent_index].knows_the_way("manure_fields") {
            self.world.worn_fields_mucked += 1;
        }

        let agent = &mut self.population.agents[agent_index];
        agent.practices.record_outcome(Practice::SpreadingMuck, worked);
        agent
            .skills
            .practise(crate::agents::SkillType::Farming, 10, turn_now);

        debug!(
            "Agent {} tipped {} spoiled units onto {:?} ({})",
            agent.id,
            tipped,
            tile_position,
            if worked { "it will tell" } else { "it will not" }
        );

        ActionResult::success()
            .with_energy_cost(3.0)
            .with_message(format!("Spread {} of muck on the ground", tipped))
    }

    /// `Action::HedgeAField`: the brush in hand laid into the edge round the
    /// camp's fields about where this one stands, begun if it was not, and
    /// closed when it holds what an edge takes (#398). Inside it, beasts do
    /// not come at a crop.
    pub(in crate::analytics) fn hedging_a_field(&mut self, agent_index: usize, turn_now: u32) -> ActionResult {
        use crate::world::{Position, ThornHedge};

        let agent = &self.population.agents[agent_index];
        if !agent.knows_the_way("hedge_fields") {
            return ActionResult::failure("Nobody here hedges fields".to_string());
        }
        let here = (agent.state.position.0, agent.state.position.1);
        let wood = agent.how_many_i_have("wood");
        if wood == 0 {
            return ActionResult::failure("No brush in hand".to_string());
        }

        let begun = self
            .world
            .thorn_hedges
            .iter()
            .position(|hedge| hedge.round_fields && hedge.closed_on.is_none() && hedge.takes_in(here));
        let which = match begun {
            Some(which) => which,
            None => {
                let a_field = self
                    .world
                    .grid
                    .get_tile(&Position::new(here.0, here.1))
                    .is_some_and(|tile| tile.terrain.is_cultivated());
                if !a_field || self.is_this_field_hedged(here) {
                    return ActionResult::failure("No open field here to hedge".to_string());
                }
                self.world.thorn_hedges.push(ThornHedge {
                    around: Position::new(here.0, here.1),
                    started: turn_now,
                    laid: 0,
                    closed_on: None,
                    round_fields: true,
                });
                self.world.thorn_hedges.len() - 1
            }
        };
        let hedge = &mut self.world.thorn_hedges[which];
        let laying = wood.min(hedge.what_it_takes().saturating_sub(hedge.laid));
        hedge.laid += laying;
        let closed = hedge.laid >= hedge.what_it_takes();
        if closed {
            hedge.closed_on = Some(turn_now);
            self.world.fields_hedged += 1;
        }

        let agent = &mut self.population.agents[agent_index];
        agent.inventory.remove_item("wood", laying);
        agent.skills.practise(crate::agents::SkillType::Farming, 3, turn_now);

        ActionResult::success()
            .with_energy_cost(6.0)
            .with_message(if closed {
                "Closed the edge round the fields".to_string()
            } else {
                format!("Laid {laying} of brush along the fields' edge")
            })
    }
}
