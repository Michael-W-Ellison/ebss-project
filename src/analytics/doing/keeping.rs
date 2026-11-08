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
    /// The store at a storehouse this one is standing at (#357), opened
    /// if it has not been yet: a covered store under the building's roof,
    /// the settlement's, where food keeps as it keeps under earth and the
    /// vermin get at it as they get at any store. There is no storehouse
    /// anywhere else - the store that every agent could reach from wherever
    /// they stood, where nothing ever went off, is gone.
    pub(in crate::analytics) fn the_store_i_am_standing_at(&mut self, agent_index: usize) -> Option<crate::world::Position> {
        let at = self.population.agents[agent_index].state.position;
        let storehouse = self
            .world
            .buildings
            .iter()
            .filter(|building| building.building_type == crate::world::BuildingType::Storehouse && building.is_completed())
            .map(|building| building.position)
            .find(|there| (there.x - at.0).abs().max((there.y - at.1).abs()) <= Self::AT_THE_STOREHOUSE)?;
        if self.world.pit_at(storehouse).is_none() {
            self.world.pits.push(crate::world::Pit { gnawed: 0, last_opened: None,
                where_it_is: storehouse,
                holds: Vec::new(),
                covered: true,
                dug: self.current_turn,
                belongs: crate::world::belonging::Belongs::ToUsAll,
            });
        }
        Some(storehouse)
    }

    /// How near a storehouse somebody stands to put things in it.
    pub(in crate::analytics) const AT_THE_STOREHOUSE: i32 = 2;

    /// `Action::Store`: what is carried put into the storehouse this one is
    /// standing at (#357), food with its clock still on it.
    pub(in crate::analytics) fn storing(&mut self, item_type: &String, amount: &u32, agent_index: usize) -> ActionResult {
        use crate::agents::storage_integration::id_to_item_type;
        let Some(kind) = id_to_item_type(item_type) else {
            return ActionResult::failure(format!("Unknown item type: {}", item_type));
        };
        let Some(store) = self.the_store_i_am_standing_at(agent_index) else {
            return ActionResult::failure("No storehouse here".to_string());
        };
        let agent = &mut self.population.agents[agent_index];
        let ids: Vec<String> = agent
            .inventory
            .get_all_items()
            .values()
            .filter(|item| item.quantity > 0 && id_to_item_type(&item.item_id) == Some(kind))
            .map(|item| item.item_id.clone())
            .collect();
        let mut wanted = *amount;
        let mut going = Vec::new();
        for id in ids {
            if wanted == 0 {
                break;
            }
            let here = agent.how_many_i_have(&id).min(wanted);
            if let Some(item) = agent.inventory.remove_item(&id, here) {
                wanted -= item.quantity;
                going.push(item);
            }
        }
        let put = *amount - wanted;
        if put == 0 {
            return ActionResult::failure(format!("No {} in inventory to store", item_type));
        }
        self.line_the_store_here(agent_index, store, put);
        self.what_opening_the_store_shows(agent_index, store);
        if let Some(pit) = self.world.pit_at_mut(store) {
            for item in going {
                pit.put_in(item);
            }
        }
        ActionResult::success()
            .with_drive_change(DriveType::Preparedness, -0.15)
            .with_energy_cost(5.0)
            .with_message(format!("Put {} {} in the storehouse", put, item_type))
    }

    /// `Action::Retrieve`: what is wanted taken out of the storehouse this
    /// one is standing at (#357), as much as goes in the pack.
    pub(in crate::analytics) fn retrieving(&mut self, item_type: &String, amount: &u32, agent_index: usize) -> ActionResult {
        use crate::agents::storage_integration::id_to_item_type;
        let Some(kind) = id_to_item_type(item_type) else {
            return ActionResult::failure(format!("Unknown item type: {}", item_type));
        };
        let Some(store) = self.the_store_i_am_standing_at(agent_index) else {
            return ActionResult::failure("No storehouse here".to_string());
        };
        let held: Vec<crate::agents::InventoryItem> = self
            .world
            .pit_at(store)
            .map(|pit| {
                pit.holds
                    .iter()
                    .filter(|item| item.quantity > 0 && id_to_item_type(&item.item_id) == Some(kind))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();
        self.what_opening_the_store_shows(agent_index, store);
        if held.is_empty() {
            return ActionResult::failure(format!("Storehouse has no {} available", item_type));
        }
        let mut wanted = *amount;
        let mut taken = 0;
        for mut item in held {
            if wanted == 0 {
                break;
            }
            item.quantity = item.quantity.min(wanted);
            let fits = self.take_what_fits(agent_index, &item);
            if fits == 0 {
                break;
            }
            if let Some(pit) = self.world.pit_at_mut(store) {
                pit.take_out(&item.item_id, fits);
            }
            wanted -= fits;
            taken += fits;
        }
        if taken == 0 {
            return ActionResult::failure("No room in the pack for it".to_string());
        }
        ActionResult::success()
            .with_drive_change(DriveType::Preparedness, -0.1)
            .with_energy_cost(5.0)
            .with_message(format!("Retrieved {} {} from the storehouse", taken, item_type))
    }

    /// `Action::Cover`.
    pub(in crate::analytics) fn covering(&mut self, what: &String, agent_index: usize, turn_now: u32) -> ActionResult {
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
            .map(|pit| pit.how_much_room_is_left())
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
        let mut putting_by = (mine.quantity - keeping_back).min(room);

        // Meat carried here for somebody else goes in to the last joint
        // owed, and the carrier's share stays in the pack (#374).
        let owed = if what == "meat" { self.owed_to_this_store(agent_index, here) } else { None };
        if let Some(owed) = owed {
            putting_by = owed.min(mine.quantity).min(room);
        }

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
                    since: turn_now,
                    as_it_was,
                    in_this,
                },
            );
        }

        if owed.is_some() {
            if let Some(hire) = self.population.agents[agent_index].working_for.as_mut() {
                hire.handed_over += putting_by;
            }
            self.world.joints_carried_for_a_share += u64::from(putting_by);
        }

        {
            let agent = &mut self.population.agents[agent_index];
            agent.inventory.remove_item(what, putting_by);
            agent
                .skills
                .practise(crate::agents::SkillType::Farming, 12, turn_now);
        }

        // A vessel goes in first, if there is one to spare (#357), or
        // whatever this one has, to somebody who knows to line a store
        // (#392)
        self.line_the_store_here(agent_index, here, putting_by);
        self.what_opening_the_store_shows(agent_index, here);

        let mut holds = 0u32;
        if let Some(pit) = self.world.pit_at_mut(here) {
            pit.put_in(going_in);
            pit.covered = true;
            holds = pit.how_much_is_in_it();
        }

        // And the man who buried it knows what he put where. The sight pass
        // notices a pit in view; this is the other way anybody learns of one,
        // and it is the more reliable: you do not forget the hole you filled.
        if holds > 0 {
            self.population.agents[agent_index]
                .memory
                .remember_how_much_is_there(
                    crate::core::memory::SpatialMemoryType::Storage,
                    (here.x, here.y, 0),
                    holds,
                );
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
    pub(in crate::analytics) fn picking_up(&mut self, what: &String, agent_index: usize, turn_now: u32) -> ActionResult {
        use crate::world::Position;

        let here = {
            let at = self.population.agents[agent_index].state.position;
            Position::new(at.0, at.1)
        };

        // A pit here is the first place to look. Taking from one is
        // not a separate verb: a person opening a store and closing it
        // again is one act, and the matrix already has stooping for a
        // thing underfoot.
        // What is in the pit here, and what one of them weighs - asked before
        // anything is lifted out of it.
        //
        // This used to take eight out of the pit and hand them to
        // `add_item` without looking at what came back. `add_item` returns
        // false when the pack is too heavy or has no free slot, and **almost
        // every caller ignores it**, so the food had already left the ground
        // and never arrived anywhere: the store was drained and nobody was
        // fed, and the action reported success while it happened.
        //
        // Directly: a man with half a unit of room standing on a pit of sixty
        // took the pit to fifty-two, his pack stayed at nought, and the model
        // said "Took 8 food out of the pit". That is the whole of the gap
        // between `PickUp` at 1.9% of a starving body's turns and `Eat` at
        // 0.4% - they open the store, and what they take out stops existing.
        //
        // The branch below it, for a thing lying on the ground, has always
        // asked first and put the thing back if it would not go in. A store
        // is not different.
        let in_the_pit = self.world.pit_at(here).and_then(|pit| {
            if matches!(what.as_str(), "bowl" | "basket") {
                return None;
            }
            pit.holds
                .iter()
                .find(|held| held.item_id == *what && held.quantity > 0)
                .cloned()
        });

        if let Some(wanted) = in_the_pit {
            // A pack full of stone makes room for supper, the same way it does
            // at a bush - see `set_down_what_is_worth_less_than_food`. This is
            // the same situation and it should not have two answers.
            // What one of them actually weighs, drying and all - not
            // `weight_per_unit` raw, which prices a dried fish at what a wet
            // one weighs and so refuses room the pack has. One spelling,
            // shared with the gate that offers this and with `total_weight`
            // itself: see `InventoryItem::what_one_of_them_weighs`.
            let each = wanted.what_one_of_them_weighs().max(f32::EPSILON);
            let asking_for = Self::what_a_day_of_this_household_is(&self.population.agents[agent_index])
                .min(wanted.quantity);
            // `set_down_what_is_worth_less_than_food` answers with the room it
            // *made*, which is nought for a pack that had room already and
            // needed to shed nothing. What is wanted here is the room there
            // is, so it is asked for afterwards.
            let _ = self.set_down_what_is_worth_less_than_food(
                agent_index,
                each * asking_for as f32,
            );
            // How much of it goes in, asked of the pack rather than worked out
            // again here. This branch used to divide the room by the weight
            // itself and then assert the result would go in - "the room was
            // measured a line ago" - which is true of the *weight* and says
            // nothing about the **slot limit**. Measured: a pack with forty
            // units of room refused a stack weighing 0.175, because it already
            // held twenty kinds of thing out of twenty and this was a
            // twenty-first. The assertion fired in
            // `a_settlement_lives_through_a_winter`.
            //
            // `take_what_fits` is the one place that answers this, weight and
            // slots together, and its own doc says so. It puts in what will go
            // and reports how much, so what comes out of the pit below is what
            // actually arrived - nothing is taken out of the store and then
            // dropped on the floor.
            let mut offered = wanted.clone();
            offered.quantity = asking_for;
            let taking = self.take_what_fits(agent_index, &offered);

            if taking == 0 {
                return ActionResult::failure(
                    "No room in the pack for what is in the store".to_string(),
                );
            }

            // Out of somebody else's store is theft (#357), found out the
            // next time its owner opens it; one's own opening finds out
            // whoever was in it before.
            //
            // **Not a campmate's.** A band shares what it puts by, and
            // taking out of the pit somebody you camp with dug is what a pit
            // at a camp is for. Booked as theft, every band robbed itself
            // twelve thousand times over forty worlds and nursed the grudges
            // (#357). Theft is out of a pit dug by somebody living elsewhere.
            let me = self.population.agents[agent_index].id;
            let whose = self.world.pit_at(here).map(|pit| self.population.agents[agent_index].may_i_use(&pit.belongs));
            let a_campmates = |owner: uuid::Uuid| {
                let mine = Self::where_this_one_lives(&self.population.agents[agent_index]);
                self.population.agents.iter().find(|agent| agent.id == owner).is_some_and(|them| {
                    let theirs = Self::where_this_one_lives(them);
                    (theirs.0 - mine.0).abs().max((theirs.1 - mine.1).abs()) <= Self::NEAR_ENOUGH_TO_SLEEP_WITH_THE_OTHERS
                })
            };
            match whose {
                Some(crate::world::belonging::Access::NotMine(owner)) if !a_campmates(owner) => {
                    let already = self.world.stores_robbed.get(&here).map_or(0, |(_, n)| *n);
                    self.world.stores_robbed.insert(here, (me, already + taking));
                }
                Some(crate::world::belonging::Access::Freely) => {
                    if let Some((thief, how_many)) = self.world.stores_robbed.remove(&here) {
                        self.population.agents[agent_index]
                            .found_out_they_took_something_of_mine(thief, "food", how_many, turn_now);
                    }
                }
                _ => {}
            }
            if let Some(pit) = self.world.pit_at_mut(here) {
                pit.take_out(what, taking);
            }
            self.what_came_out_of_the_store += taking as u64;
            self.population.agents[agent_index].took_from_the_store_at = Some(self.current_turn);

            let agent = &mut self.population.agents[agent_index];
            debug!("Agent {} took {taking} {what} out of the pit", agent.id);

            // Food in the hand is not food in the belly, and answers no hunger.
            // This took a tenth off it, which is exactly the size of a thing
            // the pattern layer notices - so taking food out of a pit was
            // credited as though it were the meal, and the meal that followed
            // was credited again. The eating is what answers; the taking out
            // goes down as the step before it (`Element::Then`). See
            // ISSUES_FOUND #259.
            return ActionResult::success()
                .with_energy_cost(1.5)
                .with_message(format!("Took {taking} {what} out of the pit"));
        }

        let Some(item) = self.world.take_off_the_ground(&here, what) else {
            return ActionResult::failure(format!("No {what} lying here"));
        };

        let how_many = item.quantity;
        let agent = &mut self.population.agents[agent_index];

        // A full pack cannot take it, and it stays where it was
        if agent.inventory.weight_capacity_remaining() < item.total_weight() {
            self.world.somebody_left_this(item, here, turn_now);
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
    pub(in crate::analytics) fn putting_down(&mut self, what: &String, agent_index: usize, turn_now: u32) -> ActionResult {
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
                    since: turn_now,
                    as_it_was,
                    in_this,
                },
            );
        }

        self.population.agents[agent_index]
            .inventory
            .remove_item(what, how_many);
        self.world.somebody_left_this(left, here, turn_now);

        debug!(
            "Agent {} put down {how_many} {what} at {here:?}",
            self.population.agents[agent_index].id
        );

        ActionResult::success()
            .with_energy_cost(1.0)
            .with_message(format!("Put down {how_many} {what}"))
    }

    /// `Action::Cache`: a hollow on ground a pit will not go into, with
    /// stones piled over it - a store, covered from the first.
    ///
    /// Stage 0 of storage, "caches in cool shade". A camp under the trees
    /// sleeps on ground that will not take a hole (#315), and up to now the
    /// only answer was to walk off it to dig. What goes in afterwards goes in
    /// the way it would go in a pit (`Action::Cover`).
    pub(in crate::analytics) fn caching(&mut self, agent_index: usize, turn_now: u32) -> ActionResult {
        use crate::world::{Pit, Position};

        let here = {
            let at = self.population.agents[agent_index].state.position;
            Position::new(at.0, at.1)
        };
        if self.world.pit_at(here).is_some() {
            return ActionResult::failure("There is a store here already".to_string());
        }
        if !self.world.grid.get_tile(&here).is_some_and(|tile| tile.terrain.is_walkable()) {
            return ActionResult::failure("Nowhere here to pile stones".to_string());
        }
        let agent = &mut self.population.agents[agent_index];
        if agent.how_many_i_have("stone") < Self::STONES_TO_A_CAIRN {
            return ActionResult::failure("Not stones enough for a cairn".to_string());
        }
        agent.inventory.remove_item("stone", Self::STONES_TO_A_CAIRN);
        agent.memory.remember_location(
            crate::core::memory::SpatialMemoryType::Storage,
            (here.x, here.y, 0),
        );
        agent
            .skills
            .practise(crate::agents::SkillType::Mining, 10, turn_now);
        let whose = agent.id;
        self.world.pits.push(Pit { gnawed: 0, last_opened: None,
            where_it_is: here,
            holds: Vec::new(),
            covered: true,
            dug: turn_now,
            belongs: crate::world::Belongs::To(whose),
        });

        ActionResult::success()
            .with_drive_change(DriveType::Preparedness, -0.1)
            .with_energy_cost(6.0)
            .with_message("Piled stones over a cache".to_string())
    }

    /// How many stones a cairn over a cache takes.
    pub(in crate::analytics) const STONES_TO_A_CAIRN: u32 = 4;

    /// `Action::PutUpRack`: poles and cord, put up where this one stands.
    pub(in crate::analytics) fn putting_up_a_rack(&mut self, agent_index: usize, turn_now: u32) -> ActionResult {
        use crate::world::Position;

        let at = self.population.agents[agent_index].state.position;
        let here = Position::new(at.0, at.1);
        let agent = &self.population.agents[agent_index];
        if !agent.knows_how_to_do("PutUpRack") {
            return ActionResult::failure("Nobody here has thought to put food up off the ground".to_string());
        }
        if self.world.a_rack_within_reach(here).is_some() {
            return ActionResult::failure("There is a rack here already".to_string());
        }
        if agent.how_many_i_have("wood") < Self::WOOD_TO_A_RACK
            || agent.how_many_i_have("lashing") < Self::LASHING_TO_A_RACK
        {
            return ActionResult::failure("Not the poles and cord for a rack".to_string());
        }
        let agent = &mut self.population.agents[agent_index];
        agent.inventory.remove_item("wood", Self::WOOD_TO_A_RACK);
        agent.inventory.remove_item("lashing", Self::LASHING_TO_A_RACK);
        agent
            .skills
            .practise(crate::agents::SkillType::Crafting, 10, turn_now);
        self.world.racks.push(here);

        ActionResult::success()
            .with_drive_change(DriveType::Preparedness, -0.1)
            .with_energy_cost(8.0)
            .with_message("Put up a rack".to_string())
    }

    /// What a rack takes: three poles and two lengths of cord.
    pub(in crate::analytics) const WOOD_TO_A_RACK: u32 = 3;
    pub(in crate::analytics) const LASHING_TO_A_RACK: u32 = 2;

    /// `Action::HangUp`: a stack of raw food hung on a rack within reach.
    ///
    /// It dries there in the sun as it would on the ground, twice as fast,
    /// and half the rain gets at it; and it stays until somebody takes it
    /// down (`Action::PickUp`, standing at the rack).
    pub(in crate::analytics) fn hanging_up(&mut self, what: &String, agent_index: usize, turn_now: u32) -> ActionResult {
        use crate::world::nutrition::{Piece, PreparationState};
        use crate::world::Position;

        let at = self.population.agents[agent_index].state.position;
        let Some(rack) = self.world.a_rack_within_reach(Position::new(at.0, at.1)) else {
            return ActionResult::failure("No rack within reach".to_string());
        };
        let agent = &self.population.agents[agent_index];
        if !agent.knows_how_to_do("HangUp") {
            return ActionResult::failure("Nobody here knows to hang it up".to_string());
        }
        if Piece::of(what) == Piece::Whole || !crate::world::World::will_this_dry(what) {
            return ActionResult::failure(format!("{what} would not dry hung up like that"));
        }
        let worth_it = agent.inventory.get_item(what).is_some_and(|item| {
            item.food_data.as_ref().is_some_and(|food| {
                food.preparation == PreparationState::Raw && food.freshness >= Self::TOO_FAR_GONE_TO_KEEP
            })
        });
        if !worth_it {
            return ActionResult::failure(format!("No {what} worth hanging up"));
        }
        let agent = &mut self.population.agents[agent_index];
        let how_many = agent.inventory.get_item(what).map_or(0, |item| item.quantity);
        let Some(item) = agent.inventory.remove_item(what, how_many) else {
            return ActionResult::failure(format!("No {what} to hang up"));
        };
        self.world.dropped.push(crate::world::Dropped {
            item,
            where_it_is: rack,
            since: turn_now,
            dried_in_the_sun: 0,
        });

        ActionResult::success()
            .with_drive_change(DriveType::Preparedness, -0.3)
            .with_energy_cost(2.0)
            .with_message(format!("Hung {how_many} {what} up on the rack"))
    }

    /// `Action::LayBedding`: boughs and brush laid where this one stands, to
    /// sleep on (#349).
    ///
    /// Stage 0 of shelter, "ground bedding". A bed to sleep on - the sleep it
    /// gives is a bed's - and off the cold ground, which is worth a layer of
    /// clothing to whoever is lying on it. Two wood of brush, and it is
    /// trodden flat in a month.
    pub(in crate::analytics) fn laying_bedding(&mut self, agent_index: usize, turn_now: u32) -> ActionResult {
        use crate::world::Position;

        let at = self.population.agents[agent_index].state.position;
        let here = Position::new(at.0, at.1);
        if self.world.bedding_at(&here) {
            return ActionResult::failure("There is bedding laid here already".to_string());
        }
        let agent = &mut self.population.agents[agent_index];

        // A hide to spare goes down first: warmer, and it lasts a season (#353)
        if let Some(hide) = Self::a_hide_to_spare_for_bedding(agent) {
            agent.inventory.remove_item(hide, 1);
            self.world.bedding.push(crate::world::Bedding { at: here, laid: turn_now, of_hide: true });
            return ActionResult::success()
                .with_energy_cost(1.0)
                .with_message("Laid a hide down to sleep on".to_string());
        }
        if agent.how_many_i_have("wood") < Self::WHAT_BEDDING_TAKES {
            return ActionResult::failure("Not brush enough for bedding".to_string());
        }
        agent.inventory.remove_item("wood", Self::WHAT_BEDDING_TAKES);
        self.world.bedding.push(crate::world::Bedding { at: here, laid: turn_now, of_hide: false });

        ActionResult::success()
            .with_energy_cost(2.0)
            .with_message("Laid bedding".to_string())
    }

    /// A worked hide this one can spare to sleep on: rawhide or leather, and
    /// only with more than one in the pack, so the one a garment or a tent
    /// is waiting on is kept (#353).
    pub(in crate::analytics) fn a_hide_to_spare_for_bedding(agent: &crate::agents::Agent) -> Option<&'static str> {
        ["rawhide", "leather"]
            .into_iter()
            .find(|hide| agent.how_many_i_have(hide) >= Self::HIDES_BEFORE_ONE_IS_SLEPT_ON)
    }

    /// How many worked hides somebody holds before laying one down to sleep
    /// on.
    pub(in crate::analytics) const HIDES_BEFORE_ONE_IS_SLEPT_ON: u32 = 3;

    /// What bedding takes: two wood of boughs and brush.
    pub(in crate::analytics) const WHAT_BEDDING_TAKES: u32 = 2;

    /// A vessel into the store at `at` before what is put by goes in, if the
    /// store is not lined already.
    ///
    /// A vessel to spare goes in, whichever keeps best: what gets at buried
    /// food is the ground itself, and a bowl or a basket in the way of it
    /// doubles what the hole is worth; a fired pot, which nothing gnaws
    /// through, was never put in until #357.
    ///
    /// Somebody who knows to line a store (#392), "woven bins", with no
    /// vessel to spare weaves a basket for it there and then, of lashings
    /// they can spare, for a store worth it. A spare was all that ever went
    /// in, and nearly half of what a people put by lay in bare earth for want
    /// of one.
    ///
    /// The first cut had them put in the only basket they had, which is what
    /// a foraging trip brings back more than an armful in, and births across
    /// forty worlds fell from 181 to 98 with #393 alongside it.
    pub(in crate::analytics) fn line_the_store_here(&mut self, agent_index: usize, at: crate::world::Position, going_in: u32) {
        if !self.world.pit_at(at).is_some_and(|pit| !pit.is_lined()) {
            return;
        }
        let knows_to = self.population.agents[agent_index].knows_the_way("line_the_store");
        let worth_it = self.world.pit_at(at).is_some_and(|pit| {
            self.is_lining_this_worth_it(&self.population.agents[agent_index], pit.how_much_is_in_it() + going_in)
        });
        let enough = 1;
        let lining = {
            let agent = &self.population.agents[agent_index];
            crate::environment::tags::EVERYTHING_THAT_KEEPS
                .iter()
                .filter(|keeps| {
                    crate::environment::tags::is_this_a(keeps.called, crate::environment::tags::Tag::FoodContainer)
                })
                .filter(|keeps| agent.how_many_i_have(keeps.called) > enough)
                .min_by(|a, b| a.how_fast_food_in_it_goes_off.total_cmp(&b.how_fast_food_in_it_goes_off))
                .map(|keeps| keeps.called)
        };
        let vessel = match lining {
            Some(vessel) => {
                self.population.agents[agent_index].inventory.remove_item(vessel, 1);
                vessel
            }
            None if knows_to
                && worth_it
                && self.population.agents[agent_index].how_many_i_have("lashing")
                    >= Self::LASHINGS_A_BASKET_FOR_A_STORE_TAKES + Self::LASHINGS_KEPT_BACK =>
            {
                self.population.agents[agent_index]
                    .inventory
                    .remove_item("lashing", Self::LASHINGS_A_BASKET_FOR_A_STORE_TAKES);
                "basket"
            }
            None => return,
        };
        if knows_to {
            self.world.stores_lined += 1;
        }
        // And a basket daubed shut as it goes in, by somebody who knows to and
        // has the clay for it (#393). Only here: a sealed basket carries
        // nothing, and made as a vessel like any other, people turned the
        // baskets they foraged with into them.
        let vessel = if vessel == "basket"
            && self.population.agents[agent_index].knows_the_way("seal_the_store")
            && self.population.agents[agent_index].how_many_i_have("clay") > 0
        {
            self.population.agents[agent_index].inventory.remove_item("clay", 1);
            self.world.stores_sealed += 1;
            "sealedbasket"
        } else {
            vessel
        };
        if let Some(pit) = self.world.pit_at_mut(at) {
            pit.put_in(if vessel == "stoneware" {
                crate::agents::InventoryItem::new_container(
                    vessel.to_string(),
                    1,
                    crate::environment::making::WHAT_A_FIRED_POT_HOLDS,
                )
            } else {
                crate::agents::InventoryItem::new_with_weight(vessel.to_string(), 1, 1.0)
            });
        }
    }

    /// What a basket for a store takes, woven there and then (#392): what a
    /// basket always takes (`making::BASKET`).
    pub(in crate::analytics) const LASHINGS_A_BASKET_FOR_A_STORE_TAKES: u32 = 2;

    /// Lashings kept back from a basket for a store, for the tools they are
    /// wanted for: a pair.
    pub(in crate::analytics) const LASHINGS_KEPT_BACK: u32 = 2;

    /// What lining a store costs, in what it would save (#394): a basket's
    /// lashings and the turn it takes to weave it, a few units of food.
    pub(in crate::analytics) const WHAT_LINING_A_STORE_COSTS: f64 = 4.0;

    /// Whether lining a bare store of `holding` is worth it to this one
    /// (#394): what a season of keeping in it would save, at what they have
    /// seen bare and lined stores lose a day, times how much waste bothers
    /// them, against what the lining costs. So a frugal man lines a store of
    /// fifty and a lazy one leaves a store of two hundred bare.
    pub(in crate::analytics) fn is_lining_this_worth_it(&self, agent: &crate::agents::Agent, holding: u32) -> bool {
        let a_day_saved = (agent.what_i_reckon_a_store_loses("bare") - agent.what_i_reckon_a_store_loses("lined")).max(0.0);
        let season = crate::environment::seasons::DAYS_PER_SEASON as f64;
        holding as f64 * a_day_saved * season * agent.how_much_waste_bothers_me() >= Self::WHAT_LINING_A_STORE_COSTS
    }

    /// The next of lining a store of the camp's that is lying bare (#394):
    /// lined where this one stands at it, else walked to, by somebody who
    /// knows to, for a store near home worth lining to them. With no vessel
    /// or lashings to spare, the lashings are made first, or the wood for
    /// them fetched: in a probe, few had either, and a world lined seventeen
    /// stores in two years.
    pub(in crate::analytics) fn a_store_to_line(
        &self,
        agent: &crate::agents::Agent,
        here: (i32, i32, i32),
    ) -> Option<crate::environment::Action> {
        use crate::environment::Action;
        if !agent.knows_the_way("line_the_store") || !agent.immediate_needs_met() {
            return None;
        }
        let a_vessel_to_spare = crate::environment::tags::EVERYTHING_THAT_KEEPS.iter().any(|keeps| {
            crate::environment::tags::is_this_a(keeps.called, crate::environment::tags::Tag::FoodContainer)
                && agent.how_many_i_have(keeps.called) > 1
        });
        let lashings_to_spare = agent.how_many_i_have("lashing")
            >= Self::LASHINGS_A_BASKET_FOR_A_STORE_TAKES + Self::LASHINGS_KEPT_BACK;
        let home = Self::where_this_one_lives(agent);
        let store = self
            .world
            .pits
            .iter()
            .filter(|pit| pit.how_it_is_shut() == "bare")
            .filter(|pit| {
                Self::within((pit.where_it_is.x, pit.where_it_is.y), home, Self::NEAR_ENOUGH_TO_SLEEP_WITH_THE_OTHERS)
            })
            .filter(|pit| self.is_lining_this_worth_it(agent, pit.how_much_is_in_it()))
            .max_by_key(|pit| pit.how_much_is_in_it())?;
        // Nothing to line it with: the lashings made, of wood, or the wood
        // fetched for them. Somebody who minds the waste goes to the trouble.
        if !a_vessel_to_spare && !lashings_to_spare {
            if agent.how_many_i_have("wood") > 0 && agent.knows_how_to_make("lashing") {
                return Some(Action::Craft { item_type: "lashing".to_string() });
            }
            return Some(Action::Gather { resource_type: "wood".to_string() });
        }
        let at = (store.where_it_is.x, store.where_it_is.y);
        if Self::within((here.0, here.1), at, 1) {
            Some(Action::LineStore)
        } else {
            Some(Action::Move { target: (at.0, at.1, here.2) })
        }
    }

    /// How much wood raising a store into a granary takes (#420): posts, a
    /// floor of poles and the frame of a thatch.
    pub(in crate::analytics) const WOOD_A_GRANARY_TAKES: u32 = 10;

    /// What raising a granary costs, in what it would save (#420): the wood
    /// fetched and the day's work, some forty units of food.
    pub(in crate::analytics) const WHAT_RAISING_A_GRANARY_COSTS: f64 = 40.0;

    /// The store of the camp's this one would raise into a granary (#420),
    /// by somebody who knows how: the fullest covered store near home, if
    /// home has no granary with room already and what the camp has put by
    /// about it, up to what a granary holds, would save over a season more
    /// than the granary costs, by what they reckon a lined store loses and
    /// how much waste bothers them.
    pub(in crate::analytics) fn a_store_to_raise(&self, agent: &crate::agents::Agent) -> Option<crate::world::Position> {
        use crate::world::Pit;
        let home = Self::where_this_one_lives(agent);
        let near_home = |pit: &&Pit| {
            Self::within((pit.where_it_is.x, pit.where_it_is.y), home, Self::NEAR_ENOUGH_TO_SLEEP_WITH_THE_OTHERS)
        };
        if self.world.pits.iter().filter(near_home).any(|pit| pit.is_a_granary() && pit.has_room()) {
            return None;
        }
        let put_by: u32 = self
            .world
            .pits
            .iter()
            .filter(near_home)
            .filter(|pit| pit.covered && !pit.is_a_granary())
            .map(|pit| pit.how_much_is_in_it())
            .sum();
        let a_day_saved = agent.what_i_reckon_a_store_loses("lined").max(0.0);
        let season = crate::environment::seasons::DAYS_PER_SEASON as f64;
        let worth = f64::from(put_by.min(Pit::WHAT_A_GRANARY_TAKES)) * a_day_saved * season * agent.how_much_waste_bothers_me();
        if worth < Self::WHAT_RAISING_A_GRANARY_COSTS {
            return None;
        }
        self.world
            .pits
            .iter()
            .filter(near_home)
            .filter(|pit| pit.covered && !pit.is_a_granary())
            .max_by_key(|pit| (pit.how_much_is_in_it(), pit.where_it_is.x, pit.where_it_is.y))
            .map(|pit| pit.where_it_is)
    }

    /// The next of raising a granary (#420): the wood for it fetched, then
    /// walked to the store and raised.
    pub(in crate::analytics) fn a_granary_to_raise(
        &self,
        agent: &crate::agents::Agent,
        here: (i32, i32, i32),
    ) -> Option<crate::environment::Action> {
        use crate::environment::Action;
        if !agent.knows_the_way("granary")
            || !agent.immediate_needs_met()
            || agent.state.years_old() < crate::agents::Agent::OLD_ENOUGH_TO_KEEP_A_KIT
        {
            return None;
        }
        let store = self.a_store_to_raise(agent)?;
        if agent.how_many_i_have("wood") < Self::WOOD_A_GRANARY_TAKES {
            return self
                .nearest_resource_within(agent, here, Self::FORAGE_RADIUS, |resource| {
                    resource.resource_type == crate::world::ResourceType::Wood
                })
                .map(|_| Action::Gather { resource_type: "wood".to_string() });
        }
        if Self::within((here.0, here.1), (store.x, store.y), 1) {
            Some(Action::RaiseGranary)
        } else {
            Some(Action::Move { target: (store.x, store.y, here.2) })
        }
    }

    /// `Action::RaiseGranary`: the covered store this one stands at raised
    /// on a floor of poles under a thatch, of the wood in hand (#420).
    pub(in crate::analytics) fn raising_a_granary(&mut self, agent_index: usize) -> crate::environment::ActionResult {
        use crate::environment::ActionResult;
        let agent = &self.population.agents[agent_index];
        if !agent.knows_the_way("granary") {
            return ActionResult::failure("Nobody here raises a granary".to_string());
        }
        if agent.how_many_i_have("wood") < Self::WOOD_A_GRANARY_TAKES {
            return ActionResult::failure("Not wood enough for a granary".to_string());
        }
        let here = agent.state.position;
        let Some(at) = self
            .world
            .pits
            .iter()
            .filter(|pit| pit.covered && !pit.is_a_granary())
            .map(|pit| pit.where_it_is)
            .find(|at| Self::within((here.0, here.1), (at.x, at.y), 1))
        else {
            return ActionResult::failure("No store here to raise".to_string());
        };
        self.what_opening_the_store_shows(agent_index, at);
        self.population.agents[agent_index].inventory.remove_item("wood", Self::WOOD_A_GRANARY_TAKES);
        if let Some(pit) = self.world.pit_at_mut(at) {
            pit.holds.push(crate::agents::InventoryItem::new_with_weight(
                crate::world::Pit::GRANARY.to_string(),
                1,
                0.0,
            ));
        }
        self.world.granaries_built += 1;
        ActionResult::success()
            .with_drive_change(crate::core::DriveType::Preparedness, -0.3)
            .with_energy_cost(10.0)
            .with_message("Raised a granary".to_string())
    }

    /// `Action::LineStore`: the bare store this one stands at lined, opened to
    /// do it (#394).
    pub(in crate::analytics) fn lining_a_store(&mut self, agent_index: usize) -> crate::environment::ActionResult {
        use crate::environment::ActionResult;
        let here = self.population.agents[agent_index].state.position;
        let Some(at) = self
            .world
            .pits
            .iter()
            .filter(|pit| pit.how_it_is_shut() == "bare")
            .map(|pit| pit.where_it_is)
            .find(|at| Self::within((here.0, here.1), (at.x, at.y), 1))
        else {
            return ActionResult::failure("No bare store here".to_string());
        };
        self.what_opening_the_store_shows(agent_index, at);
        self.line_the_store_here(agent_index, at, 0);
        if !self.world.pit_at(at).is_some_and(|pit| pit.is_lined()) {
            return ActionResult::failure("Nothing to line it with".to_string());
        }
        ActionResult::success()
            .with_drive_change(crate::core::DriveType::Preparedness, -0.15)
            .with_energy_cost(4.0)
            .with_message("Lined a store".to_string())
    }

    /// What opening a store shows (#392, #394): how much it held, for how
    /// long, and what vermin have carried off since it was last opened, by how
    /// it is shut. Somebody who opens bare stores and lined ones often enough
    /// sees which keeps better (`Agent::what_the_stores_have_shown_me`); and
    /// somebody who lines their stores and still finds them gnawed thinks to
    /// daub the next one shut (#393).
    pub(in crate::analytics) fn what_opening_the_store_shows(&mut self, agent_index: usize, at: crate::world::Position) {
        let now = self.current_turn;
        let Some(pit) = self.world.pit_at_mut(at) else {
            return;
        };
        let how = pit.how_it_is_shut();
        let gnawed = std::mem::take(&mut pit.gnawed);
        let since = pit.last_opened.replace(now).unwrap_or(pit.dug);
        let days = now.saturating_sub(since) as f64 / crate::environment::seasons::TICKS_PER_DAY as f64;
        let held = (pit.how_much_is_in_it() + gnawed) as f64;
        if how == "open" || held <= 0.0 {
            return;
        }
        let agent = &mut self.population.agents[agent_index];
        agent.what_i_saw_of_a_store(how, held * days, gnawed as f64);
        if agent.what_the_stores_have_shown_me() {
            debug!("Agent {} saw the bare stores lose what the lined ones kept", agent.id);
        }
        // Gnawed through a lining often enough, somebody who lines their
        // stores and grows a harvest thinks to get the store up off the
        // ground where the burrowers cannot come at it (#420)
        if gnawed > 0 && how == "lined" && agent.knows_the_way("line_the_store") && agent.knows_how_to_do("TillSoil") {
            agent.skills.made_one(crate::agents::Agent::A_LINED_STORE_GNAWED_BY_A_FARMER);
            if agent.skills.times_made(crate::agents::Agent::A_LINED_STORE_GNAWED_BY_A_FARMER)
                >= crate::agents::Agent::LINED_STORES_GNAWED_BEFORE_A_GRANARY
            {
                agent.it_comes_to_me("know:granary", crate::agents::Agent::LINED_STORES_GNAWED_BEFORE_A_GRANARY);
            }
        }
        // Gnawed through a lining, and somebody who lines their stores thinks
        // to daub the next one shut (#393)
        if gnawed > 0 && how == "lined" && agent.knows_the_way("line_the_store") {
            agent.skills.made_one(crate::agents::Agent::A_LINED_STORE_GNAWED);
            if agent.skills.times_made(crate::agents::Agent::A_LINED_STORE_GNAWED)
                >= crate::agents::Agent::LINED_STORES_GNAWED_BEFORE_SEALING
            {
                agent.it_comes_to_me("know:seal_the_store", crate::agents::Agent::LINED_STORES_GNAWED_BEFORE_SEALING);
            }
        }
    }
}
