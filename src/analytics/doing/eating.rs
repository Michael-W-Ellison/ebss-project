// src/analytics/doing/eating.rs
//! What a body does with food once it has it.
//!
//! Eating, and the four ways of keeping a thing long enough to eat it later
//! - cooking, boiling, salting, drying - and the one way of finding out
//! whether it can be eaten at all.
//!
//! One method per `Action` variant, called from the dispatcher in
//! [`super::execute_action`]. The bodies are as they were when all fifty-two
//! lived in one five-thousand-line `match`; what changed is that a verb can
//! now be found, read and altered without scrolling past the other fifty-one.

use super::super::Simulation;
use crate::agents::physiology;
use crate::core::DriveType;
use crate::environment::ActionResult;
use crate::world::nutrition::CookingOutcome;
use crate::world::{EatResult, NutritionalContent, Position};
use log::debug;
use rand::Rng;

impl Simulation {
    /// A sitting down to eat off what is in the hand, and how many of it that
    /// comes to.
    ///
    /// **Eating never consults the pack.** A mouth is not a rucksack: a man
    /// with his arms full can still put a handful of berries in it. This is
    /// the one place the arithmetic lives, because two verbs end with food in
    /// somebody's hand - `Eat` off a patch, and `Gather` that found no room -
    /// and they had better agree about what a meal is.
    ///
    /// How many items a sitting comes to depends on what they are: four fish
    /// or sixteen handfuls of leaf come to the same supper, which is the whole
    /// of what caloric density means here. It stops at a third of a day's
    /// energy or at a full stomach, whichever comes first, and never at
    /// nought - somebody who has walked to a bush gets a mouthful even with a
    /// stomach that says otherwise.
    pub(in crate::analytics) fn a_sitting_from_the_hand(
        &mut self,
        agent_index: usize,
        what: crate::world::ItemType,
        in_the_hand: u32,
    ) -> (u32, crate::world::nutrition::NutritionalContent) {
        let nutrition = self
            .food_database
            .get(&what)
            .map(|template| template.base_nutrition)
            .unwrap_or_else(|| NutritionalContent::new(20.0, 5.0, 35.0, 0.8));

        let now = self.current_tick;
        let agent = &mut self.population.agents[agent_index];

        agent.nutrition.consume(&nutrition);
        agent.state.eat(now, nutrition.energy);

        let worth = physiology::what_a_unit_of_this_is_worth(nutrition.energy);
        let mut eaten = 0u32;
        let mut energy_in = 0.0f32;
        while eaten < in_the_hand && energy_in < physiology::WHAT_A_SITTING_AIMS_AT {
            let went_down = agent
                .state
                .physiology
                .eat(physiology::UNITS_IN_ONE_ITEM, worth);
            if went_down <= 0.0 {
                break;
            }
            energy_in += went_down * worth;
            eaten += 1;
        }
        if eaten == 0 {
            eaten = 1;
        }

        // Foraged fruit and berries carry water too
        if nutrition.water_content > 0.3 {
            if let Some(thirst) = agent.drives.get_mut(DriveType::Thirst) {
                thirst.decrease(nutrition.water_content * 0.1);
            }
        }

        (eaten.min(in_the_hand.max(1)), nutrition)
    }

    /// `Action::Eat`.
    pub(in crate::analytics) fn eating(&mut self, food_type: &String, agent_index: usize, rng: &mut rand::rngs::StdRng) -> ActionResult {
        // A full stomach will not take more, however much the reserve
        // wants it. Somebody who has gone without cannot put it right
        // in one sitting; it takes as many days to come back as a
        // stomach holds meals. See `agents::physiology`.
        if !self.population.agents[agent_index]
            .state
            .physiology
            .room_for_another_mouthful()
        {
            return ActionResult::failure("Too full to eat".to_string());
        }

        // PRIORITY 1: eat food the agent is already carrying.
        //
        // Agents gather food into their inventory long before they are
        // hungry; without this they would starve while fully stocked.
        // `find_best_food_to_eat` is the whole answer. There used to be an
        // `.or_else` here reaching for the literal item id "food", because the
        // search could not see a stack with no nutrition data on it - so this
        // could eat an untracked stack called "food" and no other, and a pack
        // of untracked grain or fish went uneaten while counting towards what
        // the agent had put by. The search sees them now; see #231.
        let agent = &mut self.population.agents[agent_index];
        let carried_food = agent.find_best_food_to_eat();

        // A sitting down to eat, not a single berry.
        //
        // A meal used to be one item worth a flat third of a day
        // whatever it was, which made caloric density meaningless: a
        // berry and a haunch of venison were the same supper. An item
        // is a handful now - `UNITS_IN_ONE_ITEM` - and what it is
        // worth is its own energy, so somebody living on leaf eats a
        // great many more of them than somebody living on fish. That
        // is the whole of what "the caloric density of food should be
        // based on the type of food" asks for, and it only means
        // anything if a meal is allowed to be several mouthfuls.
        //
        // It stops at whichever comes first: a third of a day's energy
        // in, no room left in the stomach, or nothing left to eat.
        if let Some(item_id) = carried_food {
            let mut energy_in = 0.0f32;
            let mut mouthfuls = 0u32;
            let mut made_sick: Option<f32> = None;
            while energy_in < physiology::WHAT_A_SITTING_AIMS_AT
                && agent.state.physiology.room_in_the_stomach()
                    >= physiology::UNITS_IN_ONE_ITEM
            {
                match agent.eat_food_item(&item_id, self.current_tick) {
                    EatResult::Success(nutrition) => {
                        let worth = physiology::what_a_unit_of_this_is_worth(
                            nutrition.energy,
                        );
                        let went_down = agent
                            .state
                            .physiology
                            .eat(physiology::UNITS_IN_ONE_ITEM, worth);
                        if went_down <= 0.0 {
                            break;
                        }
                        energy_in += went_down * worth;
                        mouthfuls += 1;
                    }
                    EatResult::MadeSick(damage) => {
                        made_sick = Some(damage);
                        break;
                    }
                    // Spoiled/NoFood: nothing more of this to eat
                    EatResult::Spoiled | EatResult::NoFood => break,
                }
            }

            if let Some(damage) = made_sick {
                if mouthfuls == 0 {
                    return ActionResult::failure(format!(
                        "Ate spoiled {} and got sick ({:.1} damage)",
                        item_id, damage
                    ));
                }
            }

            if mouthfuls > 0 {
                debug!(
                    "Agent {} ate {} of carried {} ({:.0} energy), reset starvation timer",
                    agent.id, mouthfuls, item_id, energy_in
                );

                return ActionResult::success()
                    .with_drive_change(DriveType::Hunger, -0.3)
                    .with_energy_cost(1.0) // Eating from inventory is cheap
                    .with_message(format!(
                        "Ate {mouthfuls} of carried {item_id} ({energy_in:.0} energy)"
                    ));
            }
            // Nothing went down - fall through to foraging below
        }

        // PRIORITY 2: forage from a nearby food resource node
        let agent = &self.population.agents[agent_index];
        let agent_pos = Position::new(
            agent.state.position.0,
            agent.state.position.1
        );

        // Look for anything edible within a 25-tile radius - and
        // weigh the walk by what this one remembers about the ground
        // it would be walking onto. A patch in a wood where this
        // agent saw wolves last month is further away than it
        // measures; one it has no bad history with is nearer. See
        // `what_everybody_saw_that_frightened_them`.
        let now = self.current_tick;
        let remembers = &self.population.agents[agent_index].exploration_knowledge;

        // What pays best, rather than what is nearest.
        //
        // This took the nearest edible thing and never asked what it
        // was worth. Spring leaf is energy six against ordinary
        // forage's twenty-five, so a unit of it is worth a quarter -
        // and a stomach that holds six hundred units and empties in
        // six hours can take in about two thousand four hundred units
        // a day, which is five hundred and seventy-six energy against
        // the fourteen hundred and forty a body burns.
        //
        // **A body living on greens starves however many greens there
        // are.** With leaf the commonest thing growing, the nearest
        // edible thing was almost always leaf, and settlements died of
        // hunger in a spring holding three thousand units of greens,
        // fifteen hundred of roots and two and a half thousand of fish,
        // having eaten a sixth of it. Roots are energy thirty and fish
        // twenty-five: either will keep somebody alive, and leaf will
        // not.
        //
        // So the walk is weighed against what is at the end of it, and
        // a root patch twenty paces off beats a leaf underfoot. Same
        // reckoning as `provision::what_foraging_costs`, which already
        // prices a trip.
        let mut nearest_food: Option<(usize, u32)> = None;
        let mut best_worth: f32 = 0.0;
        for (i, resource) in self.world.resources.iter().enumerate() {
            if Self::edible_item_for(resource.resource_type).is_some() && resource.amount > 0 {
                let distance = agent_pos.distance_to(&resource.position);
                if distance <= Self::FORAGE_RADIUS {
                    let bad = remembers.how_bad_is_it_there(resource.position, now);
                    let felt = distance
                        + (bad * Self::WHAT_A_BAD_PLACE_ADDS_TO_A_WALK) as u32;

                    let energy = Self::edible_item_for(resource.resource_type)
                        .and_then(|kind| self.food_database.get(&kind))
                        .map(|t| t.base_nutrition.energy)
                        .unwrap_or(physiology::ENERGY_OF_ORDINARY_FOOD);
                    let costs = crate::agents::provision::what_foraging_costs(
                        felt,
                        physiology::how_much_work_this_food_is(energy),
                    );
                    let worth = Self::what_this_patch_is_worth(
                        energy,
                        resource.amount,
                        felt,
                        costs,
                    );

                    if worth > best_worth {
                        best_worth = worth;
                        nearest_food = Some((i, felt));
                    }
                }
            }
        }

        if let Some((food_index, _)) = nearest_food {
            // Strip the patch, do not pick one berry off it.
            //
            // This took exactly one portion however far it had walked,
            // ate it standing there and went home empty-handed, so a
            // settlement lived hand to mouth for ever: measured over
            // four hundred turns, two thousand one hundred and
            // ninety-nine gather trips put wood, cotton, clay and iron
            // in packs and **not one item of food**, because the only
            // path food ever took out of the ground was this one and
            // this one ate what it took. Nothing was carried, so
            // nothing could be stored, so no pit ever held a winter;
            // and every meal cost a walk, which is why an agent with
            // the whole of spring standing round it took in nine
            // hundred units a day against the fourteen hundred and
            // forty it burned.
            //
            // The Gather branch was taught this and this one was not -
            // the same lesson written down twice and applied once. See
            // the armful reasoning there.
            let today = self.world.climate.calendar.day_of_year;
            let here = self.world.resources[food_index].resource_type;
            let armful = Self::what_a_trip_brings_back(here, today, rng);
            let harvested = self.world.resources[food_index].harvest(armful);

            if harvested > 0 {
                let agent = &mut self.population.agents[agent_index];

                // Foraged food carries real nutrition, so eating it
                // refills the nutritional reserves that metabolism
                // draws down rather than only the felt-energy value.
                let foraged_item = Self::edible_item_for(
                    self.world.resources[food_index].resource_type,
                )
                .unwrap_or(crate::world::ItemType::Food);

                let nutrition = self
                    .food_database
                    .get(&foraged_item)
                    .map(|template| template.base_nutrition)
                    .unwrap_or_else(|| NutritionalContent::new(20.0, 5.0, 35.0, 0.8));

                agent.nutrition.consume(&nutrition);
                agent.state.eat(self.current_tick, nutrition.energy);

                // A sitting down to eat off the armful, and how much
                // of it that is depends on what it is: four fish or
                // sixteen handfuls of leaf come to the same supper.
                let worth = physiology::what_a_unit_of_this_is_worth(nutrition.energy);
                let mut eaten_here = 0u32;
                let mut energy_in = 0.0f32;
                while eaten_here < harvested
                    && energy_in < physiology::WHAT_A_SITTING_AIMS_AT
                {
                    let went_down = agent
                        .state
                        .physiology
                        .eat(physiology::UNITS_IN_ONE_ITEM, worth);
                    if went_down <= 0.0 {
                        break;
                    }
                    energy_in += went_down * worth;
                    eaten_here += 1;
                }
                if eaten_here == 0 {
                    eaten_here = 1;
                }

                // Foraged fruit and berries carry water too
                if nutrition.water_content > 0.3 {
                    if let Some(thirst) = agent.drives.get_mut(DriveType::Thirst) {
                        thirst.decrease(nutrition.water_content * 0.1);
                    }
                }

                debug!(
                    "Agent {} foraged and ate food, restored {:.1} energy, reset starvation timer",
                    agent.id, nutrition.energy
                );

                // One portion goes down here; the rest of the armful
                // goes home in the pack. That is what turns a meal
                // into a larder: the next two days of food are
                // already carried, `find_best_food_to_eat` finds them
                // at a cost of one instead of a walk, and
                // `what_food_i_can_spare` finally has something to
                // bury. What will not fit stays on the bush - see
                // ISSUES #165.
                let left_in_the_hand = harvested.saturating_sub(eaten_here);
                if left_in_the_hand > 0 {
                    use crate::agents::InventoryItem;
                    let mut carried = InventoryItem::new_with_weight(
                        Self::gathered_as(here).unwrap_or("food").to_string(),
                        left_in_the_hand,
                        crate::agents::provision::WHAT_A_HANDFUL_OF_FOOD_WEIGHS,
                    );
                    carried.food_data = self
                        .food_database
                        .create_food_data(&foraged_item, self.current_tick);
                    // What fits goes in and the rest goes back on the bush.
                    // This was `add_item`, which is all or nothing, so a man
                    // with room for ten and an armful of fourteen took none of
                    // them and walked home empty. See #118.
                    let went_in = self.take_what_fits(agent_index, &carried);
                    self.food_items_into_packs += went_in as u64;

                    let back = left_in_the_hand.saturating_sub(went_in);
                    if back > 0 {
                        // Back on the bush, not on the ground: the patch is
                        // as it was and nothing has been lost. Counted apart
                        // from what is genuinely left behind - see
                        // `what_went_back_on_the_bush`.
                        self.world.resources[food_index].put_it_back(back);
                        self.what_went_back_on_the_bush =
                            self.what_went_back_on_the_bush.saturating_add(back as u64);
                    }
                }

                // What the trip actually cost: the walk both ways, and
                // the work of getting this particular food out of the
                // ground or off the bone. It was a flat five whatever
                // the agent did, so a patch across the valley cost the
                // same as the bush at the door and nobody had a reason
                // to prefer the near one. See
                // `provision::what_foraging_costs`.
                let paces = agent_pos
                    .distance_to(&self.world.resources[food_index].position);
                let cost = crate::agents::provision::what_foraging_costs(
                    paces,
                    physiology::how_much_work_this_food_is(nutrition.energy),
                );

                ActionResult::success()
                    .with_drive_change(DriveType::Hunger, -0.3)
                    .with_energy_cost(cost)
                    .with_message(format!("Ate {} and restored {:.1} energy", food_type, nutrition.energy))
            } else {
                ActionResult::failure("Food source was empty".to_string())
            }
        } else {
            // No food nearby, agent fails to eat
            self.forget_nearby_food_memories(agent_index);
            ActionResult::failure("No food sources nearby".to_string())
        }
    }

    /// `Action::Cook`.
    pub(in crate::analytics) fn cooking(&mut self, food_type: &String, agent_index: usize, rng: &mut rand::rngs::StdRng) -> ActionResult {
        // And somebody old enough to be trusted with a fire.
        //
        // "Age 5-10: child agents can request food from a parent agent when
        // hungry and eat any wild food found. Age 10-15: ... **and cook raw
        // food into cooked food for consumption**." Which is to say that under
        // ten they cannot, and until now a three-year-old could put a haunch
        // on the fire.
        //
        // Refused here rather than only in the decision, on the same reasoning
        // as `is_ground_a_pit_will_go_in`: a rule that lives only in the
        // wanting layer is a rule anything reaching the verb another way can
        // walk straight past.
        if self.population.agents[agent_index].state.years_old() < Self::OLD_ENOUGH_TO_COOK {
            return ActionResult::failure("Too young to cook".to_string());
        }

        // Cooking needs a fire, and the agent has to be standing at it
        let agent_pos = self.population.agents[agent_index].state.position;
        let fire = self.nearest_fire_from(agent_pos, Self::FIRE_REACH, true);

        let (fire_id, _) = match fire {
            Some(fire) => fire,
            None => return ActionResult::failure("No lit fire within reach".to_string()),
        };

        // Which of the things it is carrying goes on the fire
        let chosen = {
            let agent = &self.population.agents[agent_index];
            match Self::cookable_item(agent, food_type) {
                Some(item_id) => item_id,
                None => {
                    return ActionResult::failure(
                        "Nothing worth putting on the fire".to_string(),
                    )
                }
            }
        };

        let item_type = crate::agents::storage_integration::id_to_item_type(&chosen);
        let outcome = item_type
            .map(|item_type| item_type.cooking_outcome())
            .unwrap_or(CookingOutcome::NotFood);

        if outcome == CookingOutcome::NotFood {
            return ActionResult::failure(format!("{} is not food", chosen));
        }

        let current_tick = self.current_tick;
        let fresh_food_data = item_type
            .and_then(|item_type| self.food_database.create_food_data(&item_type, current_tick));

        let agent = &mut self.population.agents[agent_index];

        // Watching a fire is a skill. Someone who has never done it
        // burns one meal in five; someone who has done it for years
        // burns none, so the same food is ruined or not depending on
        // who is standing over it.
        let practice = agent
            .skills
            .get_skill_if_exists(crate::agents::SkillType::Cooking)
            .map(|skill| skill.level)
            .unwrap_or(-10);
        let attentive = rng.gen::<f32>() >= Self::burn_chance(practice);

        let outcome = if attentive {
            outcome
        } else {
            CookingOutcome::Ruins
        };

        // Only what fits over the flames goes on. Cooking a whole pack
        // at once would mean one lapse of attention costing everything
        // an agent had gathered, and would leave it with no raw food
        // to fall back on.
        let carried = agent
            .inventory
            .get_item(&chosen)
            .map(|item| item.quantity)
            .unwrap_or(0);
        // How much of it fits over the flames, which is a question
        // about how small it was cut. Cut into strips and most of a
        // pack is ready at the end of one turn; left as joints and it
        // takes several.
        let over_the_flames = crate::world::nutrition::Piece::of(&chosen)
            .how_many_fit_over_a_fire();
        let quantity = carried.min(Self::COOK_BATCH).min(over_the_flames);

        if quantity == 0 {
            return ActionResult::failure(format!("No {} to cook", chosen));
        }

        let mut batch = match agent.inventory.remove_item(&chosen, quantity) {
            Some(batch) => batch,
            None => return ActionResult::failure(format!("No {} to cook", chosen)),
        };

        // Food gathered before the nutrition system knew about it can
        // reach the fire without any food data at all
        if batch.food_data.is_none() {
            batch.food_data = fresh_food_data;
        }

        match batch.food_data.as_mut() {
            Some(food) => {
                food.cook(outcome);
            }
            None => {
                agent.inventory.add_item(batch);
                return ActionResult::failure(format!("{} is not food", chosen));
            }
        }

        // What comes off the fire is a different thing from what went
        // on, and an agent carries the two side by side: a stack holds
        // one preparation state, so cooked fish cannot share an entry
        // with the raw fish still in the pack.
        batch.item_id = Self::prepared_item_id(&chosen, outcome == CookingOutcome::Improves);

        if !agent.inventory.add_item(batch) {
            return ActionResult::failure(format!(
                "Nowhere to put {} {} coming off the fire",
                quantity, chosen
            ));
        }

        agent
            .skills
            .gain_experience(crate::agents::SkillType::Cooking, 25);

        // What is on the fire is what the neighbours smell
        let _ = self
            .world
            .add_to_heat_source(&fire_id, chosen.clone(), quantity);

        if outcome == CookingOutcome::Improves {
            debug!("Agent {} cooked {} {}", self.population.agents[agent_index].id, quantity, chosen);

            ActionResult::success()
                .with_energy_cost(2.0)
                .with_message(format!("Cooked {} {}", quantity, chosen))
        } else if attentive {
            debug!(
                "Agent {} ruined {} {}: a fire is no good to it",
                self.population.agents[agent_index].id, quantity, chosen
            );

            ActionResult::failure(format!(
                "Ruined {} {}: a fire is no good to it",
                quantity, chosen
            ))
        } else {
            debug!(
                "Agent {} burnt {} {}",
                self.population.agents[agent_index].id, quantity, chosen
            );

            ActionResult::failure(format!("Burnt {} {}", quantity, chosen))
        }
    }

    /// `Action::Boil`.
    pub(in crate::analytics) fn boiling(&mut self, agent_index: usize) -> ActionResult {
        // The sea boiled down for what is in it.
        //
        // A coast is worth living on for this, and it is the only
        // route to salt for a people with no flat and no seam. It
        // wants three things at once - salt water within reach, a
        // fire already going, and something to boil it in - which is
        // why it is a thing a settled people does and a wandering one
        // does not.
        let where_i_am = self.population.agents[agent_index].state.position;

        if self.salt_water_within_reach(where_i_am).is_none() {
            return ActionResult::failure("No salt water within reach".to_string());
        }

        if self
            .nearest_fire_from(where_i_am, Self::WITHIN_REACH_OF_THE_HEARTH, true)
            .is_none()
        {
            return ActionResult::failure("No lit fire to boil it over".to_string());
        }

        let tick_now = self.current_tick;
        let agent = &mut self.population.agents[agent_index];

        // What a pot of sea water comes to when the water has gone,
        // which is not much - that is the whole of why salt is dear.
        let came_out = Self::WHAT_A_POT_OF_THE_SEA_LEAVES;

        agent.inventory.add_item(crate::agents::InventoryItem::new_with_weight(
            "salt".to_string(),
            came_out,
            0.2,
        ));
        agent.skills.practise(crate::agents::SkillType::Cooking, 8, tick_now);
        agent.lessons.record_particular("boil", true);
        agent.found_out_how_to("salt");

        ActionResult::success()
            .with_drive_change(DriveType::Preparedness, -0.15)
            .with_energy_cost(9.0)
            .with_message(format!("Boiled the sea down for {came_out} salt"))
    }

    /// `Action::Salt`.
    pub(in crate::analytics) fn salting(&mut self, what: &String, agent_index: usize) -> ActionResult {
        use crate::world::nutrition::PreparationState;

        let agent = &mut self.population.agents[agent_index];

        if agent.how_many_i_have("salt") < Self::WHAT_IT_TAKES_TO_SALT_A_LOT {
            return ActionResult::failure("No salt to rub into it".to_string());
        }

        if crate::world::nutrition::Piece::of(what)
            == crate::world::nutrition::Piece::Whole
        {
            return ActionResult::failure(format!(
                "{what} would have to be cut up before it would take salt"
            ));
        }

        let Some(item) = agent.inventory.get_item_mut(what) else {
            return ActionResult::failure(format!("No {what} to salt"));
        };

        let Some(food) = item.food_data.as_mut() else {
            return ActionResult::failure(format!("{what} is not food"));
        };

        if food.preparation != PreparationState::Raw {
            return ActionResult::failure(format!("That {what} is already seen to"));
        }

        if food.freshness < Self::TOO_FAR_GONE_TO_KEEP {
            return ActionResult::failure(format!("That {what} is past saving"));
        }

        let now = self.current_tick;
        food.set_preparation(PreparationState::Salted, now);

        // And what becomes of it afterwards, which is the only thing
        // about salting that is worth knowing. The salting itself is
        // over in a turn; whether the meat is still good in a week is
        // the question, and it stays in the pack where its owner can
        // see it.
        // What is in the pack weighs what it weighs, and preparing a
        // thing changes that - the cached total has to be told.
        agent.inventory.recalculate_weight();

        let watch_it = crate::agents::wondering::Watched::of(
            agent.inventory.get_item(what).expect("it is in there"),
        );
        let asking = agent.would_i_wonder_what_becomes_of(
            crate::agents::wondering::Wondering::SALTING_IT,
            what,
        );

        agent.inventory.remove_item("salt", Self::WHAT_IT_TAKES_TO_SALT_A_LOT);
        agent.lessons.record_particular("salt", true);

        if asking {
            let where_i_am = {
                let at = self.population.agents[agent_index].state.position;
                crate::world::Position::new(at.0, at.1)
            };
            let in_this = {
                let agent = &self.population.agents[agent_index];
                self.what_it_is_like_here(agent, agent.state.position)
            };

            self.population.agents[agent_index].now_i_wonder(
                crate::agents::wondering::Wondering {
                    did: crate::agents::wondering::Wondering::SALTING_IT.to_string(),
                    what: what.to_string(),
                    where_it_is: where_i_am,
                    since: now,
                    as_it_was: watch_it,
                    in_this,
                },
            );
        }

        ActionResult::success()
            .with_drive_change(DriveType::Preparedness, -0.2)
            .with_energy_cost(3.0)
            .with_message(format!("Salted the {what}"))
    }

    /// `Action::Dry`.
    pub(in crate::analytics) fn drying(&mut self, what: &String, agent_index: usize, tick_now: u32) -> ActionResult {
        use crate::world::nutrition::PreparationState;

        let over_a_fire = self
            .nearest_fire_from(
                self.population.agents[agent_index].state.position,
                Self::WITHIN_REACH_OF_THE_HEARTH,
                true,
            )
            .is_some();

        let agent = &mut self.population.agents[agent_index];

        // Nobody is born knowing this. It has to be watched once -
        // somebody's cut fish left out in the sun, keeping where a
        // whole one would have turned. See `who_saw_that_dry`.
        if !agent
            .what_i_found_out()
            .contains(Self::THAT_LAYING_IT_OUT_KEEPS_IT)
        {
            return ActionResult::failure(
                "Nobody here knows what laying it out would do".to_string(),
            );
        }

        // And a whole beast does not dry however long you leave it
        // out. Laying a carcass in the sun is how you make carrion.
        if crate::world::nutrition::Piece::of(what)
            == crate::world::nutrition::Piece::Whole
        {
            return ActionResult::failure(format!(
                "{what} would have to be cut up before it would dry"
            ));
        }

        let Some(item) = agent.inventory.get_item_mut(what) else {
            return ActionResult::failure(format!("No {what} to dry"));
        };

        let Some(food) = item.food_data.as_mut() else {
            return ActionResult::failure(format!("{what} is not food"));
        };

        // Only raw food is worth drying. Anything already preserved
        // is done, and anything cooked has had its chance.
        if food.preparation != PreparationState::Raw {
            return ActionResult::failure(format!("That {what} is already seen to"));
        }

        // And it has to still be worth keeping. You cannot dry
        // something that has already turned - all that gets you is
        // dry carrion.
        if food.freshness < Self::TOO_FAR_GONE_TO_KEEP {
            return ActionResult::failure(format!("That {what} is past saving"));
        }

        // In the air, or in the smoke of a fire. Smoke is quicker and
        // works in any weather; laid out in the open it keeps longer,
        // and it is the only one of the two a people without fire can
        // do at all.
        let how = if over_a_fire {
            PreparationState::Smoked
        } else {
            PreparationState::Dried
        };

        food.set_preparation(how, tick_now);

        let how_many = item.quantity;

        // Drying takes the water out, and water is most of what meat
        // weighs. A pack of dried strips is a third of the pack of raw
        // joints it was, which is the second thing preserving buys.
        agent.inventory.recalculate_weight();

        agent
            .skills
            .practise(crate::agents::SkillType::Cooking, 14, tick_now);

        debug!("Agent {} {} {how_many} {what}", agent.id, how.name());

        ActionResult::success()
            .with_drive_change(DriveType::Preparedness, -0.4)
            .with_energy_cost(Self::WHAT_DRYING_COSTS)
            .with_message(format!("{} {how_many} {what}", how.name()))
    }

    /// `Action::Taste`.
    pub(in crate::analytics) fn tasting(&mut self, agent_index: usize, rng: &mut rand::rngs::StdRng) -> ActionResult {
        use crate::world::Position;

        let agent_position = self.population.agents[agent_index].state.position;
        let here = Position::new(agent_position.0, agent_position.1);

        let Some(index) = self.world.resources.iter().position(|resource| {
            resource.position == here
                && resource.resource_type == crate::world::ResourceType::StrangePlant
                && resource.amount > 0
        }) else {
            return ActionResult::failure("Nothing here to try".to_string());
        };

        let kind = self.world.resources[index].kind;
        let feeds_you = self.world.does_this_one_feed_you(kind);

        self.world.resources[index].harvest(1);

        let agent = &mut self.population.agents[agent_index];
        agent.now_i_know_that_plant(kind, feeds_you);

        let result = if feeds_you {
            // It is food. Not much of a meal - one mouthful of a
            // strange plant is a mouthful - but the man is no worse for
            // it and the people have one more thing to eat.
            agent
                .nutrition
                .consume(&crate::world::nutrition::NutritionalContent {
                    energy: Self::WHAT_ONE_MOUTHFUL_IS_WORTH,
                    protein: 1.0,
                    micronutrients: 2.0,
                    water_content: 5.0,
                });

            debug!("Agent {} found that plant {kind} is food", agent.id);

            ActionResult::success()
                .with_drive_change(DriveType::Curiosity, -0.4)
                .with_drive_change(DriveType::Hunger, -0.05)
                .with_energy_cost(1.0)
                .with_message(format!("Tried plant {kind}: it is food"))
        } else {
            // It is not. What that costs runs from a bad afternoon to
            // everything, which is what makes the trying a real choice
            // rather than a free lookup.
            let harm = rng.gen_range(
                Self::WHAT_A_BAD_PLANT_DOES.0..=Self::WHAT_A_BAD_PLANT_DOES.1,
            );
            agent.take_damage(harm);

            debug!(
                "Agent {} was poisoned by plant {kind} ({harm:.0} damage, {:.0} health left)",
                agent.id, agent.state.health
            );

            ActionResult::failure(format!("Tried plant {kind}: it is poison"))
                .with_drive_change(DriveType::Curiosity, -0.4)
                .with_energy_cost(6.0)
        };

        // And whoever was standing about learns it too, without paying
        // for it. This is the whole value of other people: one man is
        // ill and forty know not to eat that.
        for onlooker in self
            .population
            .agents
            .iter_mut()
            .filter(|agent| agent.state.is_alive)
        {
            let apart = (onlooker.state.position.0 - here.x).abs()
                + (onlooker.state.position.1 - here.y).abs();

            if apart <= Self::CLOSE_ENOUGH_TO_SEE_IT_COME_UP {
                onlooker.now_i_know_that_plant(kind, feeds_you);
            }
        }

        result
    }

    /// `Action::Treat`.
    ///
    /// Somebody takes something for what ails them, or gives it to somebody
    /// who is ill. **It eases and it does not cure** - see
    /// `crate::environment::remedies` - and the whole of what it buys is some
    /// of the week back.
    ///
    /// Before this there was no treatment of any kind in the model. `Herbs`
    /// spawned, were gathered, became `ItemType::Herbs`, taught Herbalism and
    /// then sat in the pack for ever: ISSUES_FOUND.md #202.
    pub(in crate::analytics) fn treating(
        &mut self,
        agent_index: usize,
        who: Option<uuid::Uuid>,
        tick_now: u32,
    ) -> ActionResult {
        // Who is being treated. Nobody is treated at a distance: a remedy has
        // to be handed over, which is why this checks the reach.
        let patient = match who {
            None => agent_index,
            Some(id) => {
                let here = self.population.agents[agent_index].state.position;
                let found = self
                    .population
                    .agents
                    .iter()
                    .position(|other| other.id == id && other.state.is_alive);
                let Some(found) = found else {
                    return ActionResult::failure("Nobody of that name here".to_string());
                };
                let there = self.population.agents[found].state.position;
                let apart = (here.0 - there.0).abs().max((here.1 - there.1).abs());
                if apart > Self::HOW_CLOSE_YOU_HAVE_TO_BE_TO_DOSE_SOMEBODY {
                    return ActionResult::failure("Too far off to hand it over".to_string());
                }
                found
            }
        };

        if !self.population.agents[patient].wants_something_for_it() {
            return ActionResult::failure("There is nothing the matter".to_string());
        }

        // The remedy comes out of the pack of whoever is doing the treating,
        // and it is chosen for what is actually wrong with the patient.
        let sort = self.population.agents[patient]
            .what_ails_me()
            .map(|ailing| ailing.what_sort_it_is());
        let Some(sort) = sort else {
            return ActionResult::failure("There is nothing the matter".to_string());
        };

        let Some(remedy) = self.what_in_this_pack_would_answer(agent_index, sort) else {
            return ActionResult::failure("Nothing in the pack for it".to_string());
        };

        // It is used up. A handful of mint is a handful of mint.
        if self.population.agents[agent_index]
            .inventory
            .remove_item(&remedy, 1)
            .is_none()
        {
            return ActionResult::failure("Nothing in the pack for it".to_string());
        }

        let eased = self.population.agents[patient]
            .take_a_remedy(&remedy, tick_now)
            .unwrap_or(0.0);

        self.population.agents[agent_index].skills.practise(
            crate::agents::SkillType::Herbalism,
            Self::WHAT_DOSING_SOMEBODY_TEACHES,
            tick_now,
        );

        // Being looked after is worth something in itself, whether or not the
        // herb was. This is the placebo and the company, and it is the reason
        // the wrong remedy is not worth nothing.
        if patient != agent_index {
            let name = self.population.agents[agent_index].id;
            self.population.agents[patient].emotions.add_happiness(
                crate::agents::EmotionSource::Agent(name),
                Self::WHAT_BEING_LOOKED_AFTER_IS_WORTH,
            );
        }

        debug!(
            "Agent {} treated {} with {remedy}, easing {eased:.3}",
            self.population.agents[agent_index].id,
            if patient == agent_index { "himself".to_string() } else { "somebody".to_string() },
        );

        if eased <= 0.0 {
            // It was a remedy, it was used up, and it did nothing that could
            // be measured. That is a real outcome and it is recorded as a
            // failure so the agent can learn it - see `Undertaking::Healing`.
            return ActionResult::failure(format!("{remedy} did nothing"))
                .with_energy_cost(Self::WHAT_DOSING_SOMEBODY_COSTS);
        }

        ActionResult::success()
            .with_energy_cost(Self::WHAT_DOSING_SOMEBODY_COSTS)
            .with_message(format!("Eased it with {remedy}"))
    }

    /// The best thing in a pack for a trouble of this sort.
    ///
    /// A taught hand knows which is which; an untaught one takes whatever is
    /// called medicine - see `Agent::what_i_have_for_it`, which this is the
    /// simulation's side of.
    fn what_in_this_pack_would_answer(
        &self,
        agent_index: usize,
        sort: crate::environment::remedies::WhatARemedyEases,
    ) -> Option<String> {
        use crate::environment::remedies;

        let agent = &self.population.agents[agent_index];
        let taught = agent
            .skills
            .get_skill_if_exists(crate::agents::SkillType::Herbalism)
            .map(|skill| skill.level > 0)
            .unwrap_or(false);

        let mut best: Option<(f32, String)> = None;
        for (id, item) in agent.inventory.get_all_items().iter() {
            if item.quantity == 0 {
                continue;
            }
            let Some(remedy) = remedies::what_this_is_good_for(id) else {
                continue;
            };
            let worth = if taught && remedy.eases != sort {
                remedy.takes_off * remedies::WHAT_THE_WRONG_REMEDY_IS_STILL_WORTH
            } else {
                remedy.takes_off
            };
            if best.as_ref().map(|(so_far, _)| worth > *so_far).unwrap_or(true) {
                best = Some((worth, id.clone()));
            }
        }

        best.map(|(_, id)| id)
    }

    /// How close you have to be to hand somebody a remedy.
    const HOW_CLOSE_YOU_HAVE_TO_BE_TO_DOSE_SOMEBODY: i32 = 2;

    /// What dosing somebody teaches about herbs.
    const WHAT_DOSING_SOMEBODY_TEACHES: u32 = 8;

    /// And what it costs: the picking is done, this is the sitting with them.
    const WHAT_DOSING_SOMEBODY_COSTS: f32 = 1.0;

    /// What being looked after is worth to somebody who is ill.
    const WHAT_BEING_LOOKED_AFTER_IS_WORTH: f32 = 0.15;
}
