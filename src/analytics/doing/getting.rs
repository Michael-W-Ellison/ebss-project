// src/analytics/doing/getting.rs
//! Taking what the country has.
//!
//! Gathering, harvesting, fishing, hunting, what a living beast gives, and
//! digging.
//!
//! One method per `Action` variant, called from the dispatcher in
//! [`super::execute_action`]. The bodies are as they were when all fifty-two
//! lived in one five-thousand-line `match`; what changed is that a verb can
//! now be found, read and altered without scrolling past the other fifty-one.

use super::super::Simulation;
use crate::agents::physiology;
use crate::core::DriveType;
use crate::environment::ActionResult;
use log::debug;
use rand::Rng;

impl Simulation {
    /// What a trip to a food plant brings back.
    ///
    /// A forager strips a bush; they do not pick a single fruit and walk
    /// home. Every edible thing used to come back one at a time - a trip to a
    /// berry patch was one berry, where a trip for flax was already an armful
    /// - so a settlement gathered its whole year one portion at a time and
    /// never had a surplus to store. The Preparedness drive knew perfectly
    /// well it wanted a winter store, `putting_food_by` knew how to bury one,
    /// and there was never anything in a pack to bury.
    ///
    /// What actually limits a trip is what a person can carry, and
    /// `Inventory::add_item` already refuses what will not fit, so this is a
    /// ceiling on the picking and not on the carrying.
    ///
    /// A basket inside the window the thing bears in, and pickings outside
    /// it: a day spent on a hedge in full fruit is not the same day's work as
    /// one spent on a picked-over one.
    ///
    /// One function because it was two: the Gather branch was taught the
    /// armful and the Eat branch was not, which cost a settlement every meal
    /// it ever carried home - see the reasoning on `eating_food`. The same
    /// lesson written down twice and applied once is this document's defect
    /// number three, and the answer to it is to write it down once.
    pub(in crate::analytics) fn what_a_trip_brings_back(
        what: crate::world::ResourceType,
        today: u32,
        rng: &mut rand::rngs::StdRng,
    ) -> u32 {
        if what.is_it_bearing(today) {
            rng.gen_range(8..=14)
        } else {
            rng.gen_range(1..=3)
        }
    }

    /// `Action::Gather`.
    pub(in crate::analytics) fn gathering(&mut self, resource_type: &String, agent_index: usize, rng: &mut rand::rngs::StdRng, tick_now: u32) -> ActionResult {
        use crate::world::{ResourceType, Position};
        use crate::agents::InventoryItem;

        let resource_type_enum = Self::what_a_gather_asks_for(resource_type);

        if resource_type_enum.is_none() {
            return ActionResult::failure(format!("Unknown resource type: {}", resource_type));
        }
        let resource_type_enum = resource_type_enum.unwrap();

        // Get agent position
        let agent = &self.population.agents[agent_index];
        let agent_pos = Position::new(
            agent.state.position.0,
            agent.state.position.1
        );

        // Look for resources within a 25-tile radius. A request for
        // food accepts anything edible, so foraging is not limited to
        // generic berries when grain or fish is what is growing here.
        let gathering_food = resource_type_enum == ResourceType::Food;

        let mut nearest_resource: Option<(usize, u32)> = None;
        let mut best_worth_gathering: f32 = 0.0;
        // What this particular person will accept as food. Everybody
        // takes berries; only somebody who has seen a strange plant
        // eaten and survived will pick one.
        let knows_it_is_food = |resource: &crate::world::ResourceNode| {
            resource.resource_type == ResourceType::StrangePlant
                && self.population.agents[agent_index].is_that_plant_food(resource.kind)
        };

        // And whether this one would put its face in it. The decision
        // layer already leaves the sea alone; this is the same
        // question asked again here, because the executor takes the
        // *nearest* water and a man walking to a stream should not
        // end up at the sea because the sea was closer.
        let would_drink_the_sea =
            self.population.agents[agent_index].would_i_drink_the_sea();
        let a_drink_this_one_would_take = |resource: &crate::world::ResourceNode| {
            if resource.resource_type != ResourceType::Water || would_drink_the_sea {
                return true;
            }
            !self
                .world
                .grid
                .get_tile(&resource.position)
                .is_some_and(|tile| tile.terrain.is_the_water_salt())
        };

        for (i, resource) in self.world.resources.iter().enumerate() {
            let matches_request = (resource.resource_type == resource_type_enum
                || (gathering_food
                    && (Self::edible_item_for(resource.resource_type).is_some()
                        || knows_it_is_food(resource))))
                && a_drink_this_one_would_take(resource);

            if matches_request && resource.amount > 0 {
                let distance = agent_pos.distance_to(&resource.position);
                if distance <= Self::FORAGE_RADIUS {
                    // A trip for food is worth what it brings back.
                    //
                    // The same reckoning the Eat branch makes: leaf is
                    // worth a quarter of ordinary forage and a root
                    // more than one, so a root patch across the meadow
                    // fills a pack that a nearer patch of greens does
                    // not. Anything that is not food is picked by
                    // nearness as before - a request for wood wants
                    // the nearest wood.
                    let worth = if gathering_food {
                        let energy = Self::edible_item_for(resource.resource_type)
                            .and_then(|kind| self.food_database.get(&kind))
                            .map(|t| t.base_nutrition.energy)
                            .unwrap_or(physiology::ENERGY_OF_ORDINARY_FOOD);
                        let costs = crate::agents::provision::what_foraging_costs(
                            distance,
                            physiology::how_much_work_this_food_is(energy),
                        );
                        Self::what_this_patch_is_worth(
                            energy,
                            resource.amount,
                            distance,
                            costs,
                        )
                    } else {
                        // A request for wood wants the nearest wood,
                        // but not the last stick of it: a seam with
                        // something in it beats one with a scraping.
                        let standing =
                            (resource.amount as f32).min(Self::AS_MUCH_AS_ONE_TRIP_TAKES);
                        standing / (distance as f32 + 1.0)
                    };

                    if worth > best_worth_gathering {
                        best_worth_gathering = worth;
                        nearest_resource = Some((i, distance));
                    }
                }
            }
        }

        if let Some((resource_index, _)) = nearest_resource {
            // The harvested node may be an edible substitute for a
            // generic food request, so classify by what was found.
            // A strange plant somebody has established is food is
            // food from here on: it goes in the pack and feeds people
            // like anything else that grows.
            let resource_type_enum = match self.world.resources[resource_index].resource_type
            {
                ResourceType::StrangePlant => ResourceType::Food,
                found => found,
            };

            // What an ordinary pair of hands brings back in a trip
            let today = self.world.climate.calendar.day_of_year;
            let ordinary = match resource_type_enum {
                ResourceType::Wood => rng.gen_range(1..=3),
                // An armful at a time, like wood: a garment's worth of
                // flax one stem per trip is a week's work
                ResourceType::Flax | ResourceType::Cotton => rng.gen_range(1..=3),
                ResourceType::Stone => rng.gen_range(1..=2),
                ResourceType::Iron => 1,

                // Food by the armful too, which is the whole of
                // whether a settlement can lay anything by.
                //
                // Every edible thing came back one at a time: a trip
                // to a berry patch was one berry, where a trip for
                // flax was already an armful on exactly the reasoning
                // written above it. A forager strips a bush, they do
                // not pick a single fruit and walk home, and a day
                // that brings back a third of a meal cannot feed
                // anybody, let alone put anything by for a winter. A
                // settlement gathered its whole year one portion at a
                // time and never had a surplus to store: the
                // Preparedness drive knew perfectly well it wanted a
                // winter store, `putting_food_by` knew how to bury
                // one, and there was never anything in a pack to bury.
                //
                // What actually limits a trip is what a person can
                // carry, and `Inventory::add_item` already refuses
                // what will not fit - so this is a ceiling on the
                // picking, not on the carrying.
                //
                // And a basket rather than an armful inside the window
                // the thing actually bears in, which is what
                // `what_a_trip_brings_back` weighs. This is the whole
                // margin a settlement has: at an armful a trip, a band
                // spending three quarters of its turns on food could
                // feed itself and never bank a winter.
                ResourceType::Food
                | ResourceType::Greens
                | ResourceType::Roots
                | ResourceType::Grain
                | ResourceType::Herbs => {
                    Self::what_a_trip_brings_back(resource_type_enum, today, rng)
                }

                _ => 1,
            };

            // And what these particular hands make of it. The comment
            // here used to say "based on resource type and skill" and
            // the skill was not consulted: a lifetime of farming
            // brought back exactly what a first day did. A practised
            // hand knows which plants are worth stripping and how to
            // take a crop without ruining what is left, and brings
            // back up to twice what a beginner does.
            let trade = Self::trade_for_gathering(resource_type_enum);
            let hand = self.population.agents[agent_index].skills.hand_for(trade);

            // And what he has in his hands while he does it. A stone
            // axe was, until now, a thing an agent counted and nothing
            // else: a man carrying one felled timber at exactly the
            // rate of a man with his bare hands.
            let tool = self.population.agents[agent_index].how_much_my_tools_help(trade);

            // And how old the hands are. A six-year-old strips a bush at
            // three tenths of what his father does, which is the working half
            // of `what_a_body_this_age_can_do` - and until now a child
            // gathered exactly what a grown man gathered while eating a fifth
            // as much.
            let years = self.population.agents[agent_index].state.what_i_can_do_for_my_age();

            let worth = ordinary as f32 * hand * tool * years;

            // Carry the fraction as a chance rather than rounding it
            // away, so that a small difference in skill still tells
            // over a season of trips
            let whole = worth.floor();
            let harvest_amount =
                (whole as u32) + u32::from(rng.gen::<f32>() < worth - whole);
            let harvest_amount = harvest_amount.max(1);

            // Harvest resource
            let where_it_grew = self.world.resources[resource_index].position;
            let harvested = {
                let node = &mut self.world.resources[resource_index];
                let taken = node.harvest(harvest_amount);

                // A spring down to its springline still gives a drink:
                // you take it from the water coming out of the ground
                // rather than from the pool, so the pool does not move
                // and nobody is turned away from a running spring.
                if taken == 0 {
                    node.a_mouthful_from_the_flow()
                } else {
                    taken
                }
            };

            // What everybody standing here can see about this patch.
            // Stripping the last of something is not a private fact:
            // whoever is near enough watches the ground go bare, and
            // that is what stops a settlement walking back to it every
            // morning for the rest of the season.
            let picked_out = self.world.resources[resource_index].amount == 0;
            let now = self.current_tick;
            if harvested > 0 {
                self.population.agents[agent_index]
                    .exploration_knowledge
                    .found_some_at(where_it_grew);
            }
            if picked_out {
                for watcher in self.population.agents.iter_mut() {
                    if !watcher.state.is_alive {
                        continue;
                    }
                    let paces = (watcher.state.position.0 - where_it_grew.x)
                        .abs()
                        .max((watcher.state.position.1 - where_it_grew.y).abs());
                    if paces <= Self::CLOSE_ENOUGH_TO_SEE_IT_COME_UP {
                        watcher
                            .exploration_knowledge
                            .found_none_at(where_it_grew, now);
                    }
                }
            }

            // What a crop off broken ground teaches. Nobody is born
            // believing that seed put in the ground on purpose comes
            // back as food; carrying an armful home off a field is the
            // evidence that settles it.
            if harvested > 0
                && self
                    .world
                    .grid
                    .get_tile(&where_it_grew)
                    .map(|tile| tile.terrain.is_cultivated())
                    .unwrap_or(false)
            {
                let agent = &mut self.population.agents[agent_index];
                agent
                    .practices
                    .record_outcome(crate::agents::practices::Practice::Farming, true);

                // And which plant it was that repaid the work. This is
                // the whole of how a people finds out that grain is
                // worth sowing and a berry bush is not: it sowed both
                // and kept count of what it carried home.
                if let Some((called, _, _)) = Self::what_can_be_sown()
                    .into_iter()
                    .find(|(_, crop, _)| *crop == resource_type_enum)
                {
                    agent
                        .lessons
                        .record_particular(&format!("sow:{called}"), true);
                }
            }

            // Stone and wood go quickly, and a trip out for timber is
            // one more trip an axe will not make again.
            if harvested > 0 && tool > 1.0 {
                if let Some(broke) =
                    self.population.agents[agent_index].wear_what_i_worked_with(trade)
                {
                    debug!(
                        "Agent {} wore out a {broke}",
                        self.population.agents[agent_index].id
                    );
                }
            }

            if harvested > 0 {
                // Water is consumed immediately (drinking), not stored
                if resource_type_enum == ResourceType::Water {
                    // And whether it is water worth drinking is a
                    // question about the ground it is standing on.
                    // Every drop in this world was fresh until now: a
                    // river, a spring and the sea were one terrain
                    // and one drink.
                    let salt = self
                        .world
                        .grid
                        .get_tile(&self.world.resources[resource_index].position)
                        .is_some_and(|tile| tile.terrain.is_the_water_salt());

                    let agent = &mut self.population.agents[agent_index];

                    // Satisfy thirst drive
                    if let Some(thirst) = agent.drives.get_mut(DriveType::Thirst) {
                        thirst.partial_satisfy(0.5);
                    }

                    // Reset dehydration counter
                    agent.state.last_drank_tick = self.current_tick;
                    agent.state.ticks_without_water = 0;

                    if salt {
                        // Salt water takes more water out of a body
                        // than it puts in, and the body finds that out
                        // twenty minutes later like any other drink.
                        agent.state.physiology.hydration =
                            (agent.state.physiology.hydration
                                - physiology::A_DRINK_IS_WORTH * 0.5)
                                .max(0.0);
                    } else {
                        agent.state.physiology.drink(physiology::A_DRINK_IS_WORTH);
                    }

                    if salt {
                        // "Even if it seems to temporarily satiate
                        // it." The thirst goes down on the tick and
                        // comes back worse for days, which is the
                        // whole shape of the mistake.
                        agent.drank_salt_water(self.current_tick);

                        return ActionResult::success()
                            .with_drive_change(DriveType::Thirst, -0.5)
                            .with_energy_cost(5.0)
                            .with_message("Drank salt water".to_string());
                    }

                    // Fill containers if agent has any
                    let filled = agent.inventory.fill_containers(harvested as f32);

                    debug!(
                        "Agent {} drank water and filled {:.1} units into containers",
                        agent.id, filled
                    );

                    return ActionResult::success()
                        .with_drive_change(DriveType::Thirst, -0.5)
                        .with_energy_cost(5.0)
                        .with_message(format!("Drank water, filled {:.1} into containers", filled));
                }

                // Add to agent inventory (non-water resources).
                //
                // One table, shared with the decision that goes
                // looking for the stuff. There were two before, and
                // they had drifted: greens and roots have been going
                // into packs as "generic" since the day they were
                // added, and clay would have done the same.
                let item_id = Self::gathered_as(resource_type_enum).unwrap_or("generic");

                let mut item = InventoryItem::new_with_weight(
                    item_id.to_string(),
                    harvested,
                    match resource_type_enum {
                        ResourceType::Wood => 2.0,     // Wood is light but bulky
                        ResourceType::Stone => 5.0,    // Stone is heavy
                        ResourceType::Iron => 8.0,     // Iron is very heavy
                        ResourceType::Food => {
                            crate::agents::provision::WHAT_A_HANDFUL_OF_FOOD_WEIGHS
                        }
                        _ => 1.0,
                    }
                );

                // Gathered food carries nutrition and spoils over time
                if let Some(item_type) = Self::edible_item_for(resource_type_enum) {
                    item.food_data = self
                        .food_database
                        .create_food_data(&item_type, self.current_tick);
                }

                let it_is_food = item.food_data.is_some();
                let went_in_the_pack =
                    self.population.agents[agent_index].inventory.add_item(item);
                if it_is_food && went_in_the_pack {
                    self.food_items_into_packs += harvested as u64;
                }
                let agent = &mut self.population.agents[agent_index];
                if went_in_the_pack {
                    // Grant skill XP based on resource type
                    let skill_type = Self::trade_for_gathering(resource_type_enum);
                    // A trip out is the commonest thing anybody does
                    // and the whole of some people's trade, so it is
                    // what the climb is sized against
                    agent.skills.practise(skill_type, 8, tick_now);

                    debug!(
                        "Agent {} gathered {} {} (total weight: {:.1}/{:.1})",
                        agent.id, harvested, item_id,
                        agent.inventory.current_weight, agent.inventory.max_weight
                    );

                    ActionResult::success()
                        .with_drive_change(DriveType::Industry, -0.15)
                        .with_energy_cost(10.0)
                        .with_message(format!("Gathered {} {}", harvested, resource_type))
                } else {
                    // What you cannot carry stays where it fell.
                    //
                    // The harvest came off the node before anything
                    // asked whether it would fit, so a full pack did
                    // not merely refuse the trip - it destroyed what
                    // had been picked. Gathering by the armful with
                    // full packs was quietly deleting most of the food
                    // a settlement took: twenty-eight thousand items
                    // gathered over a run left six hundred in a pit and
                    // a hundred and fifty in packs, and the rest went
                    // nowhere at all. ISSUES #165 states this principle
                    // and never reached this branch.
                    self.world.resources[resource_index].put_it_back(harvested);
                    ActionResult::failure("Inventory full - cannot carry more".to_string())
                }
            } else {
                // Including a spring that has given what it has this
                // hour. Exempting water from this - on the reasoning
                // that a spring is running again in ten ticks and
                // should not be written off for half a season - was
                // measured and was **worse**: the failure rate went up
                // rather than down, because a man who does not
                // remember the spring was low walks back to it and is
                // refused again. Remembering where the water was not
                // is what sends him to the next one.
                {
                    let now = self.current_tick;
                    self.population.agents[agent_index]
                        .exploration_knowledge
                        .found_none_at(where_it_grew, now);
                }
                ActionResult::failure("Resource source was empty".to_string())
            }
        } else {
            // No source in range. Water can still be drunk from a
            // waterskin, which is the whole point of carrying one -
            // an agent crossing dry ground should not go thirsty with
            // a full flask on its belt.
            if resource_type_enum == ResourceType::Water {
                let current_tick = self.current_tick;
                let agent = &mut self.population.agents[agent_index];

                if agent.inventory.available_water() > 0.0 {
                    let drunk = agent.drink_water(1.0);

                    if drunk {
                        agent.state.drink(current_tick);

                        debug!("Agent {} drank from its own container", agent.id);

                        return ActionResult::success()
                            .with_drive_change(DriveType::Thirst, -0.2)
                            .with_energy_cost(1.0)
                            .with_message("Drank from a carried container".to_string());
                    }
                }
            }

            if resource_type_enum == ResourceType::Food {
                self.forget_nearby_food_memories(agent_index);
            }

            ActionResult::failure(format!("No {} sources nearby", resource_type))
        }
    }

    /// `Action::Hunt`.
    pub(in crate::analytics) fn hunting(&mut self, animal_id: &uuid::Uuid, weapon: &Option<String>, agent_index: usize, rng: &mut rand::rngs::StdRng, tick_now: u32) -> ActionResult {
        // Get species data first (clone to avoid borrow issues)
        let species = {
            if let Some(animal) = self.world.animals.get(animal_id) {
                if !animal.is_alive() {
                    return ActionResult::failure("Animal is already dead".to_string());
                }
                if animal.is_domesticated {
                    return ActionResult::failure("Cannot hunt domesticated animals".to_string());
                }

                // You have to be near enough to throw something at it.
                // Without this an agent could kill a deer on the far
                // side of the map without leaving where it stood.
                let agent_position = self.population.agents[agent_index].state.position;
                let reach = (animal.position.0 - agent_position.0)
                    .abs()
                    .max((animal.position.1 - agent_position.1).abs());

                if reach > Self::HUNT_REACH {
                    return ActionResult::failure(format!(
                        "Too far to hunt: {} tiles away",
                        reach
                    ));
                }

                let species_id = animal.species_id.clone();
                match self.world.animals.get_species(&species_id) {
                    Some(s) => s.clone(),
                    None => return ActionResult::failure("Unknown animal species".to_string()),
                }
            } else {
                return ActionResult::failure("Animal not found".to_string());
            }
        };

        // Now get mutable reference to animal
        if let Some(animal) = self.world.animals.get_mut(animal_id) {

            // Calculate success based on agent skill, weapon, and mount.
            //
            // This used to read MeleeCombat and have no floor, which
            // made hunting self-defeating: an untrained agent has that
            // skill at -10 and 0.5 + (-10 x 0.05) is zero, so the first
            // kill an agent ever made created the skill and left it
            // unable to hunt for the rest of its life. It reads the
            // Hunting skill now, which existed and had no callers, and
            // never falls below a fifth.
            let agent = &self.population.agents[agent_index];
            let hunting_skill = agent
                .skills
                .get_skill_if_exists(crate::agents::skills::SkillType::Hunting)
                .map(|s| s.level)
                .unwrap_or(-10);
            // A spear in the hand, which is the whole of stone-age
            // hunting. `weapon` is the older flag and still counts;
            // what is in the pack counts for more, and counts for
            // less as it wears.
            let spear = agent.how_much_my_tools_help(crate::agents::skills::SkillType::Hunting);

            // Something in the hand, for anything bigger than a hare.
            //
            // "Hunting any larger animal requires at least a spear...
            // Stones can be used to kill small animals, but slings
            // make stones more efficient." So there is a size below
            // which bare hands and a thrown stone will do, and above
            // which they will not, and it was not being asked at all:
            // an agent with nothing in its hands could bring down an
            // ox by walking up to it.
            if species.health > Self::AS_BIG_AS_A_STONE_WILL_KILL
                && agent.what_i_have_to_work_with(
                    crate::agents::skills::SkillType::Hunting,
                ).is_none()
            {
                return ActionResult::failure(format!(
                    "Nothing in hand to bring down a {}",
                    species.name
                ))
                .with_energy_cost(2.0);
            }
            let carried_flag: f32 = if weapon.is_some() { 0.2 } else { 0.0 };
            let weapon_bonus = carried_flag.max((spear - 1.0) * 0.25);

            // Get mounted combat bonus (hunting from horseback is advantageous!)
            let mount_bonus = agent.transport.mounted_combat_bonus();

            // Hunting is slow work. It used to land six throws in ten
            // for anybody at all, which made a deer a thing you walked
            // up to rather than a thing you stalked; a stone-age hunt
            // is mostly missing. What makes the difference is the
            // spear and the hand that throws it, not the walking up.
            let success_prob = (Self::A_THROW_THAT_TELLS
                + (hunting_skill as f32 * 0.03)
                + weapon_bonus
                + mount_bonus)
                .clamp(0.1_f32, 0.9_f32);

            if rng.gen_bool(success_prob as f64) {
                // Successful hunt - damage the animal.
                //
                // A third of what it can take, not two thirds: one
                // clean throw did not kill a bull, and a hunt that
                // ends on the first hit is not a hunt.
                let base_damage = species.health * Self::WHAT_ONE_THROW_TAKES_OUT_OF_IT;
                let combat_multiplier = 1.0 + mount_bonus;
                let damage = base_damage * combat_multiplier;
                animal.take_damage(damage);
                // A throw at a deer is one more throw the shaft will
                // not take. Twenty-five or so, and it is firewood.
                let wore_out = spear > 1.0;

                // If killed, get drops
                let mut items_gained = Vec::new();
                if !animal.is_alive() {
                    for drop in &species.drops {
                        if rng.gen_bool(drop.drop_chance as f64) {
                            let quantity = rng.gen_range(drop.min_quantity..=drop.max_quantity);
                            items_gained.push(crate::environment::ItemStack {
                                material_id: drop.material_id.clone(),
                                quantity,
                            });
                        }
                    }

                    let knife = self.population.agents[agent_index]
                        .how_much_my_tools_help(
                            crate::agents::skills::SkillType::Leatherworking,
                        );
                    let butchered = self.butcher(&items_gained, knife);
                    let where_it_fell = {
                        let at = self.population.agents[agent_index].state.position;
                        crate::world::Position::new(at.0, at.1)
                    };
                    self.into_the_pack_or_on_the_ground(
                        agent_index,
                        butchered,
                        where_it_fell,
                    );
                    let agent = &mut self.population.agents[agent_index];
                    let _ = &agent;

                    // Both tools are one job further through their
                    // lives: the spear that was thrown and the flake
                    // that took the carcass apart.
                    if wore_out {
                        agent.wear_what_i_worked_with(
                            crate::agents::skills::SkillType::Hunting,
                        );
                    }
                    if knife > 1.0 {
                        agent.wear_what_i_worked_with(
                            crate::agents::skills::SkillType::Leatherworking,
                        );
                    }

                    // Increase hunting skill
                    let agent = &mut self.population.agents[agent_index];
                    agent
                        .skills
                        .practise(crate::agents::skills::SkillType::Hunting, 30, tick_now);

                    let mut result = ActionResult::success()
                        .with_drive_change(DriveType::Hunger, -0.4)
                        .with_energy_cost(Self::WHAT_A_THROW_COSTS)
                        .with_experience(5.0)
                        .with_message(format!("Successfully hunted {} and obtained materials", species.name));

                    // Add all items gained
                    for item in items_gained {
                        result = result.with_item_gained(item);
                    }
                    result
                } else {
                    let agent = &mut self.population.agents[agent_index];
                    if wore_out {
                        agent.wear_what_i_worked_with(
                            crate::agents::skills::SkillType::Hunting,
                        );
                    }
                    agent
                        .skills
                        .practise(crate::agents::skills::SkillType::Hunting, 10, tick_now);

                    // A wounded animal is not a meal. It used to
                    // answer a tenth of a hunger for nothing at all,
                    // which is a hunt that pays whether or not it
                    // works.
                    ActionResult::success()
                        .with_energy_cost(Self::WHAT_A_THROW_COSTS)
                        .with_message(format!("Wounded {} but it escaped", species.name))
                }
            } else {
                // You learn something from the ones that get away
                self.population.agents[agent_index]
                    .skills
                    .practise(crate::agents::skills::SkillType::Hunting, 10, tick_now);

                // And a throw that misses is a spear on the ground
                // somewhere out past where the animal was. Half the
                // time it is close enough to walk over and pick up
                // and half the time it is in the bracken; either way
                // it is not in the hand any more, which is what makes
                // a missed throw cost something besides the walk.
                //
                // This is the state-change half of `throw` in the verb
                // matrix: what leaves the hand goes somewhere.
                if spear > 1.0 && rng.gen_bool(Self::HOW_OFTEN_A_MISS_LOSES_THE_SHAFT) {
                    let stood = self.population.agents[agent_index].state.position;
                    let fell = crate::world::Position::new(
                        stood.0 + rng.gen_range(-3..=3),
                        stood.1 + rng.gen_range(-3..=3),
                    );

                    let thrown = self.population.agents[agent_index]
                        .inventory
                        .get_item("spear")
                        .cloned();

                    if let Some(mut thrown) = thrown {
                        thrown.quantity = 1;
                        self.population.agents[agent_index]
                            .inventory
                            .remove_item("spear", 1);
                        self.world.somebody_left_this(thrown, fell, tick_now);

                        debug!(
                            "Agent {} threw and missed; the spear is at {fell:?}",
                            self.population.agents[agent_index].id
                        );
                    }
                }

                // A rabbit runs. A boar turns round.
                let fights_back = matches!(
                    species.behavior,
                    crate::environment::AnimalBehavior::Aggressive
                        | crate::environment::AnimalBehavior::Defensive
                        | crate::environment::AnimalBehavior::Territorial
                );

                if fights_back && species.attack_damage > 0.0 {
                    let agent = &mut self.population.agents[agent_index];
                    agent.take_damage(species.attack_damage);

                    return ActionResult::failure(format!(
                        "{} turned on the hunter ({:.0} damage)",
                        species.name, species.attack_damage
                    ))
                    .with_energy_cost(Self::WHAT_A_THROW_COSTS);
                }

                ActionResult::failure(format!("{} escaped", species.name))
                    .with_energy_cost(Self::WHAT_A_THROW_COSTS)
            }
        } else {
            ActionResult::failure("Animal not found".to_string())
        }
    }

    /// `Action::CollectAnimalProduct`.
    pub(in crate::analytics) fn collecting_from_a_beast(&mut self, animal_id: &uuid::Uuid, agent_index: usize, tick_now: u32) -> ActionResult {
        // Get species data first (clone to avoid borrow issues)
        let species = {
            if let Some(animal) = self.world.animals.get(animal_id) {
                if !animal.is_alive() {
                    return ActionResult::failure("Animal is dead".to_string());
                }
                if !animal.is_domesticated {
                    return ActionResult::failure("Can only collect from domesticated animals".to_string());
                }
                if !animal.is_mature() {
                    return ActionResult::failure("Animal is not yet mature enough to produce".to_string());
                }

                let species_id = animal.species_id.clone();
                match self.world.animals.get_species(&species_id) {
                    Some(s) => s.clone(),
                    None => return ActionResult::failure("Unknown animal species".to_string()),
                }
            } else {
                return ActionResult::failure("Animal not found".to_string());
            }
        };

        if species.living_products.is_empty() {
            return ActionResult::failure(format!("{} does not produce any products", species.name));
        }

        // Now get mutable reference to animal
        if let Some(animal) = self.world.animals.get_mut(animal_id) {
            // Check which products are ready
            let mut collected_products = Vec::new();
            for product in &species.living_products {
                if let Some(timer) = animal.product_timers.get(&product.material_id) {
                    if *timer == 0 {
                        // Product is ready
                        collected_products.push(crate::environment::ItemStack {
                            material_id: product.material_id.clone(),
                            quantity: product.quantity,
                        });

                        // Reset timer
                        animal.product_timers.insert(product.material_id.clone(), product.production_time);
                    }
                }
            }

            if !collected_products.is_empty() {
                // Milk and eggs are food, and were arriving with no nutrition
                // on them at all - so they counted towards what somebody had
                // put by, could not be eaten, and never spoiled. The same
                // defect as the trade path; see #232. Anything the food
                // database knows gets its clock started here, exactly as a
                // gathered armful does.
                let now = self.current_tick;
                let clocks: Vec<_> = collected_products
                    .iter()
                    .map(|stack| {
                        crate::agents::storage_integration::id_to_item_type(
                            &stack.material_id,
                        )
                        .and_then(|kind| self.food_database.create_food_data(&kind, now))
                    })
                    .collect();

                let agent = &mut self.population.agents[agent_index];
                for (item_stack, clock) in collected_products.iter().zip(clocks) {
                    use crate::agents::InventoryItem;
                    let mut item = InventoryItem::new_with_weight(
                        item_stack.material_id.clone(),
                        item_stack.quantity,
                        1.0, // Animal products are generally light
                    );
                    item.food_data = clock;
                    agent.inventory.add_item(item);
                }

                // Practice animal husbandry (Farming skill)
                let agent = &mut self.population.agents[agent_index];
                agent.skills.practise(crate::agents::skills::SkillType::Farming, 2, tick_now);

                let products_str = collected_products.iter()
                    .map(|p| format!("{} {}", p.quantity, p.material_id))
                    .collect::<Vec<_>>()
                    .join(", ");

                let mut result = ActionResult::success()
                    .with_drive_change(DriveType::Industry, -0.2)
                    .with_energy_cost(5.0)
                    .with_message(format!("Collected {} from {}", products_str, species.name));

                // Add all collected products
                for product in collected_products {
                    result = result.with_item_gained(product);
                }
                result
            } else {
                ActionResult::failure("No products ready for collection yet".to_string())
            }
        } else {
            ActionResult::failure("Animal not found".to_string())
        }
    }

    /// `Action::HarvestPlant`.
    pub(in crate::analytics) fn harvesting_a_plant(&mut self, plant_id: &uuid::Uuid, agent_index: usize, rng: &mut rand::rngs::StdRng, tick_now: u32) -> ActionResult {
        // Get species data first (clone to avoid borrow issues)
        let species = {
            if let Some(plant) = self.world.plants.get(plant_id) {
                if !plant.is_harvestable {
                    return ActionResult::failure("Plant is not ready for harvest".to_string());
                }
                if plant.has_been_harvested && !plant.is_cultivated {
                    return ActionResult::failure("Plant has already been harvested".to_string());
                }

                let species_id = plant.species_id.clone();
                match self.world.plants.get_species(&species_id) {
                    Some(s) => s.clone(),
                    None => return ActionResult::failure("Unknown plant species".to_string()),
                }
            } else {
                return ActionResult::failure("Plant not found".to_string());
            }
        };

        // Now get mutable reference to plant
        if let Some(plant) = self.world.plants.get_mut(plant_id) {

            // Harvest the plant
            let drops = plant.harvest(&species);

            if !drops.is_empty() {
                let mut items_gained = Vec::new();

                // Generate items from drops
                for drop in &drops {
                    let quantity = rng.gen_range(drop.min_quantity..=drop.max_quantity);
                    items_gained.push(crate::environment::ItemStack {
                        material_id: drop.material_id.clone(),
                        quantity,
                    });
                }

                // Add to agent inventory.
                //
                // Anything edible off a plant carries a clock, the
                // same as anything edible picked off the ground. This
                // path did not attach one, so a settlement that
                // harvested its own plants filled up with food that
                // could never go off - and, worse, one such stack
                // swallowed every honest one that was merged into it.
                // See ISSUES_FOUND #61.
                let now = self.current_tick;
                let clocks: Vec<Option<crate::world::nutrition::FoodData>> = items_gained
                    .iter()
                    .map(|stack| {
                        crate::agents::storage_integration::id_to_item_type(
                            &stack.material_id,
                        )
                        .and_then(|kind| self.food_database.create_food_data(&kind, now))
                    })
                    .collect();

                let agent = &mut self.population.agents[agent_index];
                for (item_stack, clock) in items_gained.iter().zip(clocks) {
                    use crate::agents::InventoryItem;
                    let mut item = InventoryItem::new_with_weight(
                        item_stack.material_id.clone(),
                        item_stack.quantity,
                        1.5, // Plant materials weight
                    );
                    item.food_data = clock;
                    agent.inventory.add_item(item);
                }

                // Practice farming skill if cultivated, gathering otherwise
                let agent = &mut self.population.agents[agent_index];
                if plant.is_cultivated {
                    agent.skills.practise(crate::agents::skills::SkillType::Farming, 2, tick_now);
                } else {
                    agent.skills.practise(crate::agents::skills::SkillType::Mining, 2, tick_now);
                }

                let items_str = items_gained.iter()
                    .map(|i| format!("{} {}", i.quantity, i.material_id))
                    .collect::<Vec<_>>()
                    .join(", ");

                let mut result = ActionResult::success()
                    .with_drive_change(DriveType::Industry, -0.2)
                    .with_energy_cost(8.0)
                    .with_experience(3.0)
                    .with_message(format!("Harvested {} from {}", items_str, species.name));

                // Add all harvested items
                for item in items_gained {
                    result = result.with_item_gained(item);
                }
                result
            } else {
                ActionResult::failure("Plant yielded nothing".to_string())
            }
        } else {
            ActionResult::failure("Plant not found".to_string())
        }
    }

    /// `Action::Excavate`.
    pub(in crate::analytics) fn excavating(&mut self, agent_index: usize, tick_now: u32) -> ActionResult {
        use crate::world::{Pit, Position};

        let here = {
            let at = self.population.agents[agent_index].state.position;
            Position::new(at.0, at.1)
        };

        if self.world.pit_at(here).is_some() {
            return ActionResult::failure("There is already a pit here".to_string());
        }

        // Ground you can break. The same question a field asks, and
        // for the same reason: you cannot dig a hole in a lake or in
        // bare rock.
        let will_dig = self
            .world
            .grid
            .get_tile(&here)
            .map(|tile| tile.terrain.can_be_tilled() || tile.terrain.is_cultivated())
            .unwrap_or(false);

        if !will_dig {
            return ActionResult::failure("Nothing to dig here".to_string());
        }

        self.world.pits.push(Pit {
            where_it_is: here,
            holds: Vec::new(),
            covered: false,
            dug: tick_now,
        });

        // What comes out of a hole. The matrix says excavating changes
        // the ground and what is held, and this is the second half.
        let agent = &mut self.population.agents[agent_index];
        agent.inventory.add_item(crate::agents::InventoryItem::new_with_weight(
            "stone".to_string(),
            Self::WHAT_COMES_OUT_OF_A_HOLE,
            1.0,
        ));
        agent
            .skills
            .practise(crate::agents::SkillType::Mining, 20, tick_now);

        debug!("Agent {} dug a pit at {here:?}", agent.id);

        // And what it cost, which depends on what was in the hand.
        //
        // A flat twenty-two whether the agent dug with a shovel or with
        // its fingers - which is most of a turn's work either way, and
        // a settlement that cannot dig cheaply cannot keep a larder.
        let shovel = self.population.agents[agent_index]
            .how_much_my_tools_help(crate::agents::SkillType::Mining);
        if shovel > 1.0 {
            self.population.agents[agent_index]
                .wear_what_i_worked_with(crate::agents::SkillType::Mining);
        }

        ActionResult::success()
            .with_drive_change(DriveType::Preparedness, -0.3)
            .with_energy_cost(Self::WHAT_DIGGING_A_PIT_COSTS / shovel.max(0.1))
            .with_message("Dug a pit".to_string())
    }

    /// `Action::Fish`.
    pub(in crate::analytics) fn fishing(&mut self, agent_index: usize, rng: &mut rand::rngs::StdRng, tick_now: u32) -> ActionResult {
        // Whether it worked is recorded by `Agent::learn_from`, off
        // this arm's own success, along with every other undertaking.
        let agent_position = self.population.agents[agent_index].state.position;

        let Some(reach) = self.reach_within_cast(agent_position) else {
            return ActionResult::failure("No water in reach".to_string());
        };

        // What the agent brings to it. A rod is worth having and a
        // practised hand is worth more, but a river in the run will
        // feed somebody who has neither.
        let skill = self.population.agents[agent_index]
            .skills
            .get_skill_if_exists(crate::agents::SkillType::Fishing)
            .map(|skill| skill.level)
            .unwrap_or(-10) as f32;
        // A rod used to be looked for here by name, and given a fifth
        // of a chance of its own. That was written when nothing in the
        // making chain produced one, so the branch had never fired;
        // now that a fishing rod is a tool like any other it is
        // counted twice, and counting it twice put the rod *above* the
        // net - "net fishing is even better" - which is the
        // duplicated-vocabulary defect inverting a ladder. The tool
        // table is the one place that says what fishing tackle is
        // worth.

        let standing = self
            .world
            .resources
            .iter()
            .find(|resource| resource.position == reach)
            .map(|resource| resource.amount)
            .unwrap_or(0);

        if standing == 0 {
            return ActionResult::failure("The reach is empty".to_string());
        }

        // How thick the water is decides most of it. A run is a run:
        // anybody standing in it comes out with something, which is
        // exactly why a fishery is worth building a life beside and a
        // deer is not. It is still slow: standing in cold water
        // waiting for something to come within reach of a thrust is
        // most of a morning for a couple of fish.
        let thickness = (standing as f32 / Self::A_GOOD_REACH).clamp(0.0, 1.0);

        // A spear is what a people with no line fishes with, and it
        // is slow work: standing in the shallows waiting for
        // something to come within reach of a thrust.
        let spear = self.population.agents[agent_index]
            .how_much_my_tools_help(crate::agents::SkillType::Fishing);

        let hand = (skill / 10.0).clamp(0.0, 0.5) + (spear - 1.0) * 0.3;
        let odds = (Self::A_THRUST_THAT_TELLS + 0.4 * thickness + hand).clamp(0.0, 0.9);

        if spear > 1.0 {
            self.population.agents[agent_index]
                .wear_what_i_worked_with(crate::agents::SkillType::Fishing);
        }

        if rng.gen::<f32>() > odds {
            return ActionResult::failure("Nothing took".to_string())
                .with_energy_cost(Self::WHAT_A_THRUST_COSTS);
        }

        // And how many come out of the water when one does.
        //
        // In proportion to the tackle, rather than in two steps. The
        // odds of a cast are capped at nine tenths, so past a certain
        // point better tackle cannot land more *often* - and "net
        // fishing is even better" than a pole has to mean something.
        // What a net does that a line cannot is take several at once.
        let caught = ((Self::FISH_PER_CAST as f32 * spear).round() as u32).max(1);

        let taken = {
            let resource = self
                .world
                .resources
                .iter_mut()
                .find(|resource| resource.position == reach);
            match resource {
                Some(resource) => {
                    let taken = caught.min(resource.amount);
                    resource.amount -= taken;
                    taken
                }
                None => 0,
            }
        };

        if taken == 0 {
            return ActionResult::failure("The reach is empty".to_string());
        }

        // A fish is not all meat. The guts and heads go straight to
        // waste, and that waste is the richest thing a farming people
        // beside a river ever get their hands on - it came out of the
        // sea rather than out of their own fields.
        let food_data = self
            .food_database
            .create_food_data(&crate::world::inventory::ItemType::Fish, self.current_tick);

        let agent = &mut self.population.agents[agent_index];
        let mut catch =
            crate::agents::InventoryItem::new_with_weight("fish".to_string(), taken, 0.8);
        catch.food_data = food_data;
        agent.inventory.add_item(catch);
        agent.state.waste_carried +=
            taken as f32 * crate::world::Soil::NUTRIENT_PER_FISH * Self::OFFAL_SHARE;
        agent
            .skills
            .practise(crate::agents::SkillType::Fishing, 12, tick_now);

        debug!("Agent {} took {} fish from {:?}", agent.id, taken, reach);

        ActionResult::success()
            .with_drive_change(DriveType::Hunger, -0.15)
            .with_drive_change(DriveType::Sustenance, -0.2)
            .with_energy_cost(Self::WHAT_A_THRUST_COSTS)
            .with_message(format!("Took {} fish out of the water", taken))
    }
}
