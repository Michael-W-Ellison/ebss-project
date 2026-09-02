// src/analytics/doing/keeping.rs
//! Carrying, and putting by.
//!
//! The store and the pit, and what a pair of hands picks up, sets down and
//! covers over.
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
    /// `Action::Store`.
    pub(in crate::analytics) fn storing(&mut self, item_type: &String, amount: &u32, agent_index: usize) -> ActionResult {
        use crate::agents::storage_integration::{
            id_to_item_type, take_from_agent_inventory,
            count_in_agent_inventory
        };

        let agent = &mut self.population.agents[agent_index];

        // Try to convert string item_type to ItemType
        if let Some(item) = id_to_item_type(item_type) {
            let available = count_in_agent_inventory(&agent.inventory, item);

            if available == 0 {
                return ActionResult::failure(format!(
                    "No {} in inventory to store", item_type
                ));
            }

            // Determine how much to deposit based on storage preferences
            let deposit_amount = (*amount).min(available);

            // Remove from agent inventory
            let (success, removed) = take_from_agent_inventory(
                &mut agent.inventory,
                item,
                deposit_amount,
            );

            if success && removed > 0 {
                // Add to world storehouse
                if let Some(existing) = self.world.storehouse_inventory.items.get_mut(&item) {
                    existing.quantity += removed;
                } else {
                    self.world.storehouse_inventory.items.insert(
                        item,
                        crate::world::inventory::Item {
                            item_type: item,
                            quantity: removed,
                            durability: 100,
                            max_durability: 100,
                        },
                    );
                }

                // Read here rather than in the block below: the timeline event
                // that wants them is behind `feature = "gui"`, and `agent` is
                // borrowed from `self` for only as long as this arm.
                #[cfg(feature = "gui")]
                let agent_id = agent.id;
                #[cfg(feature = "gui")]
                let agent_pos = (agent.state.position.0, agent.state.position.1);

                debug!(
                    "Agent {} deposited {} {} to storehouse (storehouse now has {})",
                    agent.id,
                    removed,
                    item_type,
                    self.world.storehouse_inventory.items.get(&item)
                        .map(|i| i.quantity)
                        .unwrap_or(0)
                );

                // Emit storehouse deposit event for timeline (only for significant deposits)
                #[cfg(feature = "gui")]
                if removed >= 3 {
                    use crate::gui::events::{SimulationEvent, SimulationEventType};
                    let event = SimulationEvent::new(
                        self.current_tick,
                        SimulationEventType::StorehouseDeposit {
                            agent_id,
                            resource: item_type.clone(),
                            amount: removed,
                        },
                        Some(agent_pos),
                    );
                    self.population.pending_events.push(event);
                }

                ActionResult::success()
                    .with_drive_change(DriveType::Preparedness, -0.15)
                    .with_energy_cost(5.0)
                    .with_message(format!(
                        "Deposited {} {} to storehouse", removed, item_type
                    ))
            } else {
                ActionResult::failure(format!(
                    "Failed to remove {} from inventory", item_type
                ))
            }
        } else {
            ActionResult::failure(format!(
                "Unknown item type: {}", item_type
            ))
        }
    }

    /// `Action::Retrieve`.
    pub(in crate::analytics) fn retrieving(&mut self, item_type: &String, amount: &u32, agent_index: usize) -> ActionResult {
        use crate::agents::storage_integration::{
            id_to_item_type, add_to_agent_inventory
        };

        let agent = &mut self.population.agents[agent_index];

        // Try to convert string item_type to ItemType
        if let Some(item) = id_to_item_type(item_type) {
            // Check storehouse inventory
            let storehouse_available = self.world.storehouse_inventory.items
                .get(&item)
                .map(|i| i.quantity)
                .unwrap_or(0);

            if storehouse_available == 0 {
                return ActionResult::failure(format!(
                    "Storehouse has no {} available", item_type
                ));
            }

            // Determine how much to retrieve
            let retrieve_amount = (*amount).min(storehouse_available);

            // Try to add to agent inventory
            let (_success, added) = add_to_agent_inventory(
                &mut agent.inventory,
                item,
                retrieve_amount,
            );

            if added > 0 {
                // Remove from world storehouse
                if let Some(existing) = self.world.storehouse_inventory.items.get_mut(&item) {
                    existing.quantity -= added;
                    if existing.quantity == 0 {
                        self.world.storehouse_inventory.items.remove(&item);
                    }
                }

                debug!(
                    "Agent {} retrieved {} {} from storehouse (storehouse now has {})",
                    agent.id,
                    added,
                    item_type,
                    self.world.storehouse_inventory.items.get(&item)
                        .map(|i| i.quantity)
                        .unwrap_or(0)
                );

                let message = if added < retrieve_amount {
                    format!(
                        "Retrieved {} {} from storehouse (inventory full, couldn't take all {})",
                        added, item_type, retrieve_amount
                    )
                } else {
                    format!("Retrieved {} {} from storehouse", added, item_type)
                };

                ActionResult::success()
                    .with_drive_change(DriveType::Preparedness, -0.1)
                    .with_energy_cost(5.0)
                    .with_message(message)
            } else {
                ActionResult::failure(format!(
                    "Inventory full, cannot retrieve {}", item_type
                ))
            }
        } else {
            ActionResult::failure(format!(
                "Unknown item type: {}", item_type
            ))
        }
    }

    /// `Action::Cover`.
    pub(in crate::analytics) fn covering(&mut self, what: &String, agent_index: usize, tick_now: u32) -> ActionResult {
        use crate::world::Position;

        let here = {
            let at = self.population.agents[agent_index].state.position;
            Position::new(at.0, at.1)
        };

        if self.world.pit_at(here).is_none() {
            return ActionResult::failure("No pit here to put it in".to_string());
        }

        let Some(mine) = self.population.agents[agent_index]
            .inventory
            .get_item(what)
            .filter(|item| item.quantity > 0)
            .cloned()
        else {
            return ActionResult::failure(format!("No {what} to put by"));
        };

        let room = self
            .world
            .pit_at(here)
            .map(|pit| crate::world::Pit::WHAT_A_PIT_TAKES - pit.how_much_is_in_it())
            .unwrap_or(0);

        if room == 0 {
            return ActionResult::failure("The pit is full".to_string());
        }

        // A person standing on their own store keeps one meal about
        // them and buries the rest. Keeping three days' food in the
        // pack while standing on the larder is nonsense - you can
        // take more out tomorrow, that is what it is for - and it was
        // what stopped anything ever being stored: measured directly,
        // `Cover` was refused 1,513 times out of 1,525 for "not
        // enough to be worth burying", because a settlement living
        // hand to mouth rarely holds more than three of anything.
        let keeping_back = Self::WHAT_A_PERSON_KEEPS_ON_THEM.min(mine.quantity);
        let putting_by = (mine.quantity - keeping_back).min(room);

        if putting_by == 0 {
            return ActionResult::failure("Not enough to be worth burying".to_string());
        }

        let mut going_in = mine.clone();
        going_in.quantity = putting_by;

        // What happens if I bury it. Nobody is born knowing that a
        // hole in the cold ground keeps food, and the answer does not
        // arrive for a week - so it is a question, remembered the same
        // way as any other.
        //
        // What makes it a *different* question from leaving something
        // on the grass is what counts as a good answer: coming back to
        // find it exactly as it went in is the entire point here, and
        // is nothing at all there.
        if self.population.agents[agent_index].would_i_wonder_what_becomes_of(
            crate::agents::wondering::Wondering::BURYING_IT,
            what,
        ) && going_in.food_data.is_some()
        {
            let as_it_was = crate::agents::wondering::Watched::of(&going_in);
            let in_this = {
                let agent = &self.population.agents[agent_index];
                self.what_it_is_like_here(agent, agent.state.position)
            };

            self.population.agents[agent_index].now_i_wonder(
                crate::agents::wondering::Wondering {
                    did: crate::agents::wondering::Wondering::BURYING_IT.to_string(),
                    what: what.to_string(),
                    where_it_is: here,
                    since: tick_now,
                    as_it_was,
                    in_this,
                },
            );
        }

        {
            let agent = &mut self.population.agents[agent_index];
            agent.inventory.remove_item(what, putting_by);
            agent
                .skills
                .practise(crate::agents::SkillType::Farming, 12, tick_now);
        }

        // A vessel goes in first, if there is one to spare. What
        // gets at buried food is the ground itself, and a bowl or a
        // basket in the way of it doubles what the hole is worth.
        let lining = {
            let agent = &self.population.agents[agent_index];
            ["bowl", "basket"]
                .into_iter()
                .find(|vessel| agent.how_many_i_have(vessel) > 1)
        };

        if let Some(vessel) = lining {
            if self.world.pit_at(here).is_some_and(|pit| !pit.is_lined()) {
                self.population.agents[agent_index]
                    .inventory
                    .remove_item(vessel, 1);

                if let Some(pit) = self.world.pit_at_mut(here) {
                    pit.put_in(crate::agents::InventoryItem::new_with_weight(
                        vessel.to_string(),
                        1,
                        1.0,
                    ));
                }
            }
        }

        if let Some(pit) = self.world.pit_at_mut(here) {
            pit.put_in(going_in);
            pit.covered = true;
        }

        debug!(
            "Agent {} buried {putting_by} {what} at {here:?}",
            self.population.agents[agent_index].id
        );

        ActionResult::success()
            .with_drive_change(DriveType::Preparedness, -0.5)
            .with_energy_cost(4.0)
            .with_message(format!("Put {putting_by} {what} by"))
    }

    /// `Action::PickUp`.
    pub(in crate::analytics) fn picking_up(&mut self, what: &String, agent_index: usize, tick_now: u32) -> ActionResult {
        use crate::world::Position;

        let here = {
            let at = self.population.agents[agent_index].state.position;
            Position::new(at.0, at.1)
        };

        // A pit here is the first place to look. Taking from one is
        // not a separate verb: a person opening a store and closing it
        // again is one act, and the matrix already has stooping for a
        // thing underfoot.
        if let Some(from_the_pit) = self
            .world
            .pit_at_mut(here)
            .and_then(|pit| {
                if matches!(what.as_str(), "bowl" | "basket") {
                    return None;
                }

                let wanted = pit
                    .holds
                    .iter()
                    .find(|held| held.item_id == *what && held.quantity > 0)
                    .cloned()?;

                let taking = Self::WHAT_A_PERSON_TAKES_OUT.min(wanted.quantity);
                pit.take_out(what, taking);

                let mut got = wanted;
                got.quantity = taking;
                Some(got)
            })
        {
            let how_many = from_the_pit.quantity;
            let agent = &mut self.population.agents[agent_index];
            agent.inventory.add_item(from_the_pit);

            debug!("Agent {} took {how_many} {what} out of the pit", agent.id);

            return ActionResult::success()
                .with_drive_change(DriveType::Hunger, -0.1)
                .with_energy_cost(1.5)
                .with_message(format!("Took {how_many} {what} out of the pit"));
        }

        let Some(item) = self.world.take_off_the_ground(&here, what) else {
            return ActionResult::failure(format!("No {what} lying here"));
        };

        let how_many = item.quantity;
        let agent = &mut self.population.agents[agent_index];

        // A full pack cannot take it, and it stays where it was
        if agent.inventory.weight_capacity_remaining()
            < item.weight_per_unit * how_many as f32
        {
            self.world.somebody_left_this(item, here, tick_now);
            return ActionResult::failure("No room for it".to_string());
        }

        agent.inventory.add_item(item);

        debug!("Agent {} picked up {how_many} {what} at {here:?}", agent.id);

        ActionResult::success()
            .with_drive_change(DriveType::Utility, -0.2)
            .with_energy_cost(1.0)
            .with_message(format!("Picked up {how_many} {what}"))
    }

    /// `Action::PutDown`.
    pub(in crate::analytics) fn putting_down(&mut self, what: &String, agent_index: usize, tick_now: u32) -> ActionResult {
        use crate::world::Position;

        let here = {
            let at = self.population.agents[agent_index].state.position;
            Position::new(at.0, at.1)
        };

        let Some(item) = self.population.agents[agent_index]
            .inventory
            .get_item(what)
            .cloned()
        else {
            return ActionResult::failure(format!("No {what} to put down"));
        };

        if item.quantity == 0 {
            return ActionResult::failure(format!("No {what} to put down"));
        }

        // Whether this is somebody asking a question or somebody
        // putting food down. The difference matters twice over: only
        // the first is worth remembering, and only the first should
        // cost a single portion rather than the whole pack.
        let asking = {
            let agent = &self.population.agents[agent_index];
            !agent.am_i_wondering_about(crate::agents::Agent::LEAVING_IT_OUT, what)
                && !agent
                    .do_i_know_what_becomes_of(crate::agents::Agent::LEAVING_IT_OUT, what)
                && item.food_data.is_some()
                && item.quantity > 1
        };

        // You leave a bit out to see what happens to it. You do not
        // tip the whole pack on the grass - which is what this did to
        // begin with, and it cost a settlement an eighth of its people
        // and a seventh of its winter store.
        let how_many = if asking { 1 } else { item.quantity };

        let mut left = item.clone();
        left.quantity = how_many;

        if asking {
            // What was it like when it was put down, and what was the
            // sky doing. Both are wanted later and neither can be
            // recovered then: by the time anybody walks back to look,
            // the thing has changed and the rain has stopped.
            let as_it_was = crate::agents::wondering::Watched::of(&left);
            let in_this = {
                let agent = &self.population.agents[agent_index];
                self.what_it_is_like_here(agent, agent.state.position)
            };

            self.population.agents[agent_index].now_i_wonder(
                crate::agents::wondering::Wondering {
                    did: crate::agents::Agent::LEAVING_IT_OUT.to_string(),
                    what: what.to_string(),
                    where_it_is: here,
                    since: tick_now,
                    as_it_was,
                    in_this,
                },
            );
        }

        self.population.agents[agent_index]
            .inventory
            .remove_item(what, how_many);
        self.world.somebody_left_this(left, here, tick_now);

        debug!(
            "Agent {} put down {how_many} {what} at {here:?}",
            self.population.agents[agent_index].id
        );

        ActionResult::success()
            .with_energy_cost(1.0)
            .with_message(format!("Put down {how_many} {what}"))
    }
}
