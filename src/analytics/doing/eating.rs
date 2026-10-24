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
    ) -> (u32, f32, crate::world::nutrition::NutritionalContent) {
        let nutrition = self
            .food_database
            .get(&what)
            .map(|template| template.base_nutrition)
            .unwrap_or_else(|| NutritionalContent::new(20.0, 5.0, 35.0, 0.8));

        let now = self.current_turn;
        let agent = &mut self.population.agents[agent_index];

        agent.nutrition.consume(&nutrition);
        agent.state.eat(now, nutrition.energy);

        let worth = physiology::what_a_unit_of_this_is_worth(nutrition.energy);
        let mut eaten = 0u32;
        let mut energy_in = 0.0f32;
        // What stops a meal is the stomach, not a target.
        //
        // This read `energy_in < WHAT_A_SITTING_AIMS_AT` and stopped at four
        // hundred and eighty whatever room was left. A sitting is what a body
        // *aims* at, not what it can hold. `physiology::eat` charges the
        // stomach in energy and returns what actually went down, so nought
        // back is the stomach saying no. See #217.
        while eaten < in_the_hand {
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

        let name = format!("{what:?}");
        *self.what_went_down.entry(name).or_default() += eaten as u64;
        self.energy_that_went_down += energy_in as f64;
        let agent = &mut self.population.agents[agent_index];

        // Foraged fruit and berries carry water too
        if nutrition.water_content > 0.3 {
            if let Some(thirst) = agent.drives.get_mut(DriveType::Thirst) {
                thirst.decrease(nutrition.water_content * 0.1);
            }
        }

        (eaten.min(in_the_hand.max(1)), energy_in, nutrition)
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
        // Nothing in hand that will go in a mouth, but a whole fish or joint
        // and no edge to cut it: hands will have to do (#300).
        let current_turn = self.current_turn;
        let database = &self.food_database;
        let agent = &mut self.population.agents[agent_index];
        if agent
            .pull_apart_by_hand(|as_food| database.create_food_data(&as_food, current_turn))
            .is_some()
        {
            debug!("Agent {} pulled a whole one apart by hand to eat", agent.id);
        }
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
            let mut went_down_here = 0u64;
            let mut made_sick: Option<f32> = None;
            // The stomach is the cap, and it is asked in energy - a mouthful
            // of this food is `UNITS_IN_ONE_ITEM` of it at what it is worth.
            // See `a_sitting_from_the_hand`.
            while agent.state.physiology.room_in_the_stomach()
                >= physiology::UNITS_IN_ONE_ITEM
                    * physiology::ENERGY_OF_ORDINARY_FOOD
            {
                match agent.eat_food_item(&item_id, self.current_turn) {
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
                        went_down_here += 1;
                    }
                    EatResult::MadeSick(damage) => {
                        made_sick = Some(damage);
                        break;
                    }
                    // Spoiled/NoFood: nothing more of this to eat
                    EatResult::Spoiled | EatResult::NoFood => break,
                }
            }

            *self.what_went_down.entry(item_id.clone()).or_default() += went_down_here;
            self.energy_that_went_down += energy_in as f64;
            let agent = &mut self.population.agents[agent_index];

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
                agent.by_what_way = Some(format!("{:?}", agent.how_this_meal_was_come_by(self.current_turn)));

                return ActionResult::success()
                    .with_drive_change(
                        DriveType::Hunger,
                        -crate::analytics::WHAT_A_FULL_SITTING_ANSWERS
                            * agent.state.physiology.what_this_meal_answers_here(energy_in),
                    )
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
        let now = self.current_turn;
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
        for i in self.nodes_this_one_knows_of(
            &self.population.agents[agent_index],
            agent_pos,
            Self::FORAGE_RADIUS,
        ) {
            let resource = &self.world.resources[i];
            if Self::edible_item_for(resource.resource_type).is_some()
                && resource.anything_to_take()
                && !resource.is_a_carcass()
            {
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
            self.looking_at_the_crop(agent_index, food_index);
            let harvested = self.world.resources[food_index].harvest(armful);
            self.world.did_that_empty_it(food_index);

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
                agent.state.eat(self.current_turn, nutrition.energy);

                // A sitting down to eat off the armful, and how much
                // of it that is depends on what it is: four fish or
                // sixteen handfuls of leaf come to the same supper.
                let worth = physiology::what_a_unit_of_this_is_worth(nutrition.energy);
                let mut eaten_here = 0u32;
                let mut energy_in = 0.0f32;
                while eaten_here < harvested
                    && energy_in < agent.state.physiology.what_a_sitting_is_for_whoever_it_feeds()
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

                // Counted as eaten, and what it carries into the body, as a
                // meal out of the pack is (#113): food eaten where it grew
                // was never counted, nor its salt, nor flesh off a carcass
                // as meat.
                let as_carried = format!("{foraged_item:?}").to_lowercase();
                agent.food_i_ate = agent.food_i_ate.saturating_add(eaten_here);
                agent.salt_in_the_body = (agent.salt_in_the_body
                    + eaten_here as f32 * crate::agents::Agent::salt_in_a_mouthful_of(&as_carried))
                    .min(1.0);
                if foraged_item == crate::world::ItemType::Meat {
                    agent.last_ate_meat = Some(self.current_turn);
                }

                *self
                    .what_went_down
                    .entry(format!("{foraged_item:?}"))
                    .or_default() += eaten_here as u64;
                self.energy_that_went_down += energy_in as f64;
                let agent = &mut self.population.agents[agent_index];

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
                agent.by_what_way = Some(format!(
                    "{:?}",
                    crate::analytics::wanting::strategy::Strategy::GatherWildFood
                ));

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
                        .create_food_data(&foraged_item, self.current_turn);
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

        // In a fired pot of water, if there is one and water to fill it: a
        // stew, a bigger batch, nothing burnt (#355). And the pot may crack.
        let agent = &self.population.agents[agent_index];
        let carried = agent.how_much_water_i_carry() >= Self::WHAT_A_BOILING_TAKES;
        let in_a_pot = agent.how_many_i_have("stoneware") > 0
            && (carried || self.drinkable_water_within_reach(agent, agent_pos));
        let cooked = if in_a_pot {
            let stewed = self.stewing_a_batch(agent_index, food_type, fire_id);
            if stewed.success {
                if carried {
                    self.population.agents[agent_index].draw_from_what_i_carry(Self::WHAT_A_BOILING_TAKES);
                }
                if rng.gen_bool(Self::A_POT_CRACKS_OVER_THE_FIRE) {
                    self.population.agents[agent_index].a_pot_breaks();
                }
            }
            stewed
        } else {
            self.cooking_a_batch(agent_index, food_type, fire_id, Self::COOK_BATCH, true, Some(rng))
        };

        // Enough suppers over a fire, and somebody buries the next one with
        // the stones out of it (#348).
        if cooked.success {
            let agent = &mut self.population.agents[agent_index];
            agent.skills.made_one(crate::agents::Agent::A_BATCH_OFF_THE_FIRE);
            if agent.skills.times_made(crate::agents::Agent::A_BATCH_OFF_THE_FIRE)
                >= crate::agents::Agent::BATCHES_BEFORE_A_PIT_OVEN
            {
                agent.found_out_how_to("do:PitOven");
            }
        }
        cooked
    }

    /// A batch of what this one carries, put to the heat of the fire
    /// `fire_id`: up to `most` of one thing, and over the flames only what
    /// fits over the flames. With `rng` somebody has to watch it and a lapse
    /// burns it; without, it is in a pit or a pot of water and nothing burns.
    /// A whole joint does not go in either way.
    fn cooking_a_batch(
        &mut self,
        agent_index: usize,
        food_type: &str,
        fire_id: uuid::Uuid,
        most: u32,
        over_the_flames: bool,
        rng: Option<&mut rand::rngs::StdRng>,
    ) -> ActionResult {
        self.a_batch_put_to_the_heat(agent_index, food_type, fire_id, most, over_the_flames, rng, false)
    }

    /// A batch simmered in a fired pot of water over the fire `fire_id`
    /// (#355): what comes out is stewed, and nothing burns.
    fn stewing_a_batch(&mut self, agent_index: usize, food_type: &str, fire_id: uuid::Uuid) -> ActionResult {
        self.a_batch_put_to_the_heat(agent_index, food_type, fire_id, Self::WHAT_A_POT_STEWS, false, None, true)
    }

    /// How much goes in a pot at once.
    pub(in crate::analytics) const WHAT_A_POT_STEWS: u32 = 10;

    /// How often a fired pot cracks over the fire: one use in forty.
    pub(in crate::analytics) const A_POT_CRACKS_OVER_THE_FIRE: f64 = 1.0 / 40.0;

    #[allow(clippy::too_many_arguments)]
    fn a_batch_put_to_the_heat(
        &mut self,
        agent_index: usize,
        food_type: &str,
        fire_id: uuid::Uuid,
        most: u32,
        over_the_flames: bool,
        rng: Option<&mut rand::rngs::StdRng>,
        in_a_pot: bool,
    ) -> ActionResult {
        // Which of the things it is carrying goes on the fire
        let chosen = {
            let agent = &self.population.agents[agent_index];
            let chosen = if in_a_pot {
                Self::stewable_item(agent, food_type)
            } else {
                Self::cookable_item(agent, food_type)
            };
            match chosen {
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
            .map(|item_type| if in_a_pot { item_type.stewing_outcome() } else { item_type.cooking_outcome() })
            .unwrap_or(CookingOutcome::NotFood);

        if outcome == CookingOutcome::NotFood {
            return ActionResult::failure(format!("{} is not food", chosen));
        }

        let current_turn = self.current_turn;
        let fresh_food_data = item_type
            .and_then(|item_type| self.food_database.create_food_data(&item_type, current_turn));

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
        // Nothing burns in a pit or in a pot of water: the heat is in the
        // stones and the stones do not flare.
        let attentive = match rng {
            Some(rng) => rng.gen::<f32>() >= Self::burn_chance(practice),
            None => true,
        };

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
        let piece = crate::world::nutrition::Piece::of(&chosen);
        let room = if over_the_flames {
            piece.how_many_fit_over_a_fire()
        } else if piece == crate::world::nutrition::Piece::Whole {
            0
        } else {
            most
        };
        let quantity = carried.min(most).min(room);

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
            Some(food) if in_a_pot => {
                food.stew(outcome);
            }
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
        if in_a_pot && outcome == CookingOutcome::Improves {
            batch.item_id = batch.item_id.replacen("cooked_", "stewed_", 1);
        }

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

        // And something to boil it in, which this never asked: a pot that
        // will stand the fire, or water in a basket or a bag brought to the
        // boil with stones out of it (#348).
        let Some(with_stones) = Self::something_to_boil_it_in(&self.population.agents[agent_index]) else {
            return ActionResult::failure("Nothing to boil it in".to_string());
        };

        let turn_now = self.current_turn;
        let agent = &mut self.population.agents[agent_index];

        // A fired pot over the fire may crack (#355)
        if !with_stones && {
            use rand::Rng;
            crate::core::dice::roll().gen_bool(Self::A_POT_CRACKS_OVER_THE_FIRE)
        } {
            agent.a_pot_breaks();
        }

        // What a pot of sea water comes to when the water has gone,
        // which is not much - that is the whole of why salt is dear.
        let came_out = Self::WHAT_A_POT_OF_THE_SEA_LEAVES;

        agent.inventory.add_item(crate::agents::InventoryItem::new_with_weight(
            "salt".to_string(),
            came_out,
            0.2,
        ));
        agent.skills.practise(crate::agents::SkillType::Cooking, 8, turn_now);
        agent.lessons.record_particular("boil", true);
        agent.found_out_how_to("salt");
        // And salt in the hand is the start of salting, which the catalogue
        // has to know about for brine to follow it
        agent.found_out_how_to("do:Salt");

        ActionResult::success()
            .with_drive_change(DriveType::Preparedness, -0.15)
            .with_energy_cost(9.0)
            .with_message(format!("Boiled the sea down for {came_out} salt"))
    }

    /// What this one has to boil water in: `Some(false)` for a fired pot
    /// over the fire, `Some(true)` for stone boiling - a basket, bag or bowl,
    /// with the stones out of the hearth's ring - and `None` for nothing.
    ///
    /// The stones are the fire's, not the pack's. Asked of the pack, nobody
    /// ever boiled anything: hardly anybody carries stones about (#348).
    pub(in crate::analytics) fn something_to_boil_it_in(agent: &crate::agents::Agent) -> Option<bool> {
        if agent.how_many_i_have("stoneware") > 0 {
            return Some(false);
        }
        let a_vessel = Self::VESSELS_TO_BOIL_IN_WITH_STONES
            .iter()
            .any(|vessel| agent.how_many_i_have(vessel) > 0);
        (agent.knows_how_to_do("StoneBoil") && a_vessel).then_some(true)
    }

    /// What will hold water while stones out of the fire boil it, which is
    /// anything that holds water and would burn over a flame.
    pub(in crate::analytics) const VESSELS_TO_BOIL_IN_WITH_STONES: [&'static str; 4] =
        ["basket", "leatherbag", "bowl", "claypot"];

    /// `Action::StoneBoil`: cooking in a vessel of water, with stones out of
    /// the fire.
    ///
    /// Stage 1 of cooking, and come to by whoever can make a basket. More
    /// at once than over the flames, and nothing burns: the heat is in the
    /// stones, which are the hearth's own. The water is what is carried,
    /// spent, or else filled from water within reach.
    pub(in crate::analytics) fn stone_boiling(&mut self, agent_index: usize) -> ActionResult {
        if self.population.agents[agent_index].state.years_old() < Self::OLD_ENOUGH_TO_COOK {
            return ActionResult::failure("Too young to cook".to_string());
        }
        let agent = &self.population.agents[agent_index];
        if !agent.knows_how_to_do("StoneBoil") {
            return ActionResult::failure("Nobody here knows to boil with stones".to_string());
        }
        if Self::something_to_boil_it_in(agent) != Some(true) {
            return ActionResult::failure("Nothing to boil it in".to_string());
        }
        let carried = agent.how_much_water_i_carry() >= Self::WHAT_A_BOILING_TAKES;
        if !carried && !self.drinkable_water_within_reach(agent, agent.state.position) {
            return ActionResult::failure("No water to boil".to_string());
        }
        let Some((fire_id, _)) = self.nearest_fire_from(agent.state.position, Self::FIRE_REACH, true) else {
            return ActionResult::failure("No lit fire to heat the stones in".to_string());
        };

        let cooked = self.cooking_a_batch(agent_index, "generic", fire_id, Self::WHAT_A_BOILING_COOKS, false, None);
        if cooked.success && carried {
            self.population.agents[agent_index].draw_from_what_i_carry(Self::WHAT_A_BOILING_TAKES);
        }
        cooked
    }

    /// How much water a boiling takes, and how much goes in it.
    pub(in crate::analytics) const WHAT_A_BOILING_TAKES: f32 = 1.0;
    pub(in crate::analytics) const WHAT_A_BOILING_COOKS: u32 = 8;

    /// `Action::PitOven`: a big batch buried with stones out of the fire.
    ///
    /// Stage 1 of cooking, found out after enough suppers over a fire. Most
    /// of a pack at once and nothing burnt, for the wood to heat the stones
    /// and a hole to put them in - and the hole gives up its own stones, as
    /// digging a store does. Asked of the pack, nobody ever had them (#348).
    pub(in crate::analytics) fn pit_roasting(&mut self, agent_index: usize) -> ActionResult {
        if self.population.agents[agent_index].state.years_old() < Self::OLD_ENOUGH_TO_COOK {
            return ActionResult::failure("Too young to cook".to_string());
        }
        let agent = &self.population.agents[agent_index];
        if !agent.knows_how_to_do("PitOven") {
            return ActionResult::failure("Nobody here knows to cook in the ground".to_string());
        }
        let here = crate::world::Position::new(agent.state.position.0, agent.state.position.1);
        if !self.is_ground_a_pit_will_go_in(here) {
            return ActionResult::failure("No ground here to dig an oven in".to_string());
        }
        if agent.how_many_i_have("wood") < Self::WOOD_FOR_A_PIT_OVEN {
            return ActionResult::failure("Not wood enough to heat the stones".to_string());
        }
        let Some((fire_id, _)) = self.nearest_fire_from(agent.state.position, Self::FIRE_REACH, true) else {
            return ActionResult::failure("No lit fire to heat the stones in".to_string());
        };

        let cooked = self.cooking_a_batch(agent_index, "generic", fire_id, Self::WHAT_A_PIT_OVEN_COOKS, false, None);
        if cooked.success {
            self.population.agents[agent_index]
                .inventory
                .remove_item("wood", Self::WOOD_FOR_A_PIT_OVEN);
            return cooked.with_energy_cost(Self::WHAT_A_PIT_OVEN_COSTS);
        }
        cooked
    }

    /// A pit oven: the wood that heats the stones, burnt; what it costs to
    /// dig and fill; and how much goes in.
    pub(in crate::analytics) const WOOD_FOR_A_PIT_OVEN: u32 = 2;
    pub(in crate::analytics) const WHAT_A_PIT_OVEN_COSTS: f32 = 8.0;
    pub(in crate::analytics) const WHAT_A_PIT_OVEN_COOKS: u32 = 20;

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

        let now = self.current_turn;
        food.set_preparation(PreparationState::Salted, now);

        // And what becomes of it afterwards, which is the only thing
        // about salting that is worth knowing. The salting itself is
        // over in a turn;
        let watch_it = crate::agents::wondering::Watched::of(
            agent.inventory.get_item(what).expect("it is in there"),
        );
        let asking = agent.would_i_wonder_what_becomes_of(
            crate::agents::wondering::Wondering::SALTING_IT,
            what,
        );

        agent.inventory.remove_item("salt", Self::WHAT_IT_TAKES_TO_SALT_A_LOT);
        // And this is the same lesson the question about it writes when the
        // answer comes back - `Wondering::called` is `salt:meat` and this was
        // a bare `salt`, so salting a joint and finding out what became of
        // the joint went into two different rows and neither ever saw the
        // other's evidence. One spelling, shared with `what_was_tried`.
        agent.lessons.record_particular(&format!("salt:{what}"), true);

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
    pub(in crate::analytics) fn drying(&mut self, what: &String, agent_index: usize, turn_now: u32) -> ActionResult {
        use crate::world::nutrition::PreparationState;

        let at = self.population.agents[agent_index].state.position;
        let by_the_hearth = self
            .nearest_fire_from(at, Self::WITHIN_REACH_OF_THE_HEARTH, true)
            .is_some();

        // Nobody is born knowing this. It has to be watched once -
        // somebody's cut fish left out in the sun, keeping where a
        // whole one would have turned. See `who_saw_that_dry`.
        if !self.population.agents[agent_index]
            .what_i_found_out()
            .contains(Self::THAT_LAYING_IT_OUT_KEEPS_IT)
        {
            return ActionResult::failure(
                "Nobody here knows what laying it out would do".to_string(),
            );
        }

        // Not in the rain. Laid out wet, a thing turns before it dries, and
        // that is what smoking over a fire is for - see `smoking` (#348).
        if self.too_wet_to_dry_here(at) {
            return ActionResult::failure("It will not dry in the wet".to_string());
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

        let how_many = match self.keeping_it(what, agent_index, PreparationState::Dried, "dried_", turn_now) {
            Ok(how_many) => how_many,
            Err(refused) => return refused,
        };

        // Dried within reach of a fire that is burning, often enough, and
        // somebody notices that what hung nearest the smoke kept best, and
        // kept through the wet. Drying beside the hearth used to give smoked
        // food to everybody, unasked; smoking is Stage 1 of preservation,
        // and found out (#348).
        if by_the_hearth {
            let agent = &mut self.population.agents[agent_index];
            agent.skills.made_one(crate::agents::Agent::DRIED_BY_THE_HEARTH);
            if agent.skills.times_made(crate::agents::Agent::DRIED_BY_THE_HEARTH)
                >= crate::agents::Agent::DRIED_BY_THE_HEARTH_BEFORE_SMOKING
            {
                agent.found_out_how_to("do:Smoke");
            }
        }

        debug!("Agent {} dried {how_many} {what}", self.population.agents[agent_index].id);

        ActionResult::success()
            .with_drive_change(DriveType::Preparedness, -0.4)
            .with_energy_cost(Self::WHAT_DRYING_COSTS)
            .with_message(format!("Dried {how_many} {what}"))
    }

    /// Whether it is too wet here for anything laid out to dry: rain or snow
    /// coming down, and no roof overhead.
    pub(in crate::analytics) fn too_wet_to_dry_here(&self, at: (i32, i32, i32)) -> bool {
        self.world.climate.weather.weather_type.precipitation_intensity() > 0.0
            && !self
                .world
                .buildings
                .iter()
                .any(|building| building.position.x == at.0 && building.position.y == at.1)
    }

    /// What drying, smoking and brining have in common: a stack of raw food
    /// still worth keeping comes out of the pack changed, and goes back in
    /// under its own name, `prefix` in front of what it was.
    ///
    /// **Under its own name**, as what comes off a fire is
    /// (`prepared_item_id`). One stack holds one preparation, and a merge
    /// takes the one that spoils faster. Dried in place, the first raw
    /// handful gathered after it turned the whole stack raw again: it spoiled
    /// at the raw rate, and weighed what raw food weighs. It had gone into the
    /// pack at its dried weight, a third of that, so by midsummer eleven
    /// people in twelve carried more than their packs could hold, up to 94
    /// against 42 (#320).
    ///
    /// Whether it is the right shape for it is the caller's to ask.
    fn keeping_it(
        &mut self,
        what: &str,
        agent_index: usize,
        how: crate::world::nutrition::PreparationState,
        prefix: &str,
        turn_now: u32,
    ) -> Result<u32, ActionResult> {
        use crate::world::nutrition::PreparationState;

        let agent = &mut self.population.agents[agent_index];
        let Some(item) = agent.inventory.get_item_mut(what) else {
            return Err(ActionResult::failure(format!("No {what} to keep")));
        };
        let Some(food) = item.food_data.as_mut() else {
            return Err(ActionResult::failure(format!("{what} is not food")));
        };

        // Only raw food is worth it. Anything already preserved is done, and
        // anything cooked has had its chance.
        if food.preparation != PreparationState::Raw {
            return Err(ActionResult::failure(format!("That {what} is already seen to")));
        }

        // And it has to still be worth keeping: dry carrion is carrion.
        if food.freshness < Self::TOO_FAR_GONE_TO_KEEP {
            return Err(ActionResult::failure(format!("That {what} is past saving")));
        }

        food.set_preparation(how, turn_now);
        let how_many = item.quantity;
        if let Some(mut kept) = agent.inventory.remove_item(what, how_many) {
            kept.item_id = format!("{prefix}{what}");
            agent.inventory.add_item(kept);
        }
        agent
            .skills
            .practise(crate::agents::SkillType::Cooking, 14, turn_now);
        Ok(how_many)
    }

    /// `Action::Smoke`: food hung in the smoke of a low fire.
    ///
    /// Stage 1 of preservation, found out by drying beside the hearth. It
    /// keeps half as well as food dried in the sun and it is the one way of
    /// keeping a thing that does not care what the sky is doing.
    pub(in crate::analytics) fn smoking(&mut self, what: &String, agent_index: usize, turn_now: u32) -> ActionResult {
        use crate::world::nutrition::{Piece, PreparationState};

        if !self.population.agents[agent_index].knows_how_to_do("Smoke") {
            return ActionResult::failure("Nobody here has found out what smoke does to it".to_string());
        }
        let at = self.population.agents[agent_index].state.position;
        if self
            .nearest_fire_from(at, Self::WITHIN_REACH_OF_THE_HEARTH, true)
            .is_none()
        {
            return ActionResult::failure("No fire to hang it over".to_string());
        }
        if Piece::of(what) == Piece::Whole {
            return ActionResult::failure(format!("{what} would have to be cut up before it would take the smoke"));
        }

        let how_many = match self.keeping_it(what, agent_index, PreparationState::Smoked, "smoked_", turn_now) {
            Ok(how_many) => how_many,
            Err(refused) => return refused,
        };

        ActionResult::success()
            .with_drive_change(DriveType::Preparedness, -0.4)
            .with_energy_cost(Self::WHAT_DRYING_COSTS)
            .with_message(format!("Smoked {how_many} {what}"))
    }

    /// `Action::Brine`: food put down in salt water in a vessel.
    ///
    /// Stage 1 of preservation beside dry salting, and come to by whoever
    /// knows that: the same salt, gone further in water, and keeping better
    /// than rubbed in.
    pub(in crate::analytics) fn brining(&mut self, what: &String, agent_index: usize, turn_now: u32) -> ActionResult {
        use crate::world::nutrition::{Piece, PreparationState};

        let agent = &self.population.agents[agent_index];
        if !agent.knows_how_to_do("Brine") {
            return ActionResult::failure("Nobody here knows to put it down in salt water".to_string());
        }
        if agent.how_many_i_have("salt") < Self::WHAT_IT_TAKES_TO_SALT_A_LOT {
            return ActionResult::failure("No salt for a brine".to_string());
        }
        if agent.how_much_water_i_carry() < Self::WHAT_A_BRINE_TAKES {
            return ActionResult::failure("Nothing with water in it to put it down in".to_string());
        }
        if Piece::of(what) == Piece::Whole {
            return ActionResult::failure(format!("{what} would have to be cut up before it would take the brine"));
        }

        let how_many = match self.keeping_it(what, agent_index, PreparationState::Pickled, "brined_", turn_now) {
            Ok(how_many) => how_many,
            Err(refused) => return refused,
        };
        let agent = &mut self.population.agents[agent_index];
        agent.inventory.remove_item("salt", Self::WHAT_IT_TAKES_TO_SALT_A_LOT);
        agent.draw_from_what_i_carry(Self::WHAT_A_BRINE_TAKES);
        agent.lessons.record_particular(&format!("brine:{what}"), true);

        ActionResult::success()
            .with_drive_change(DriveType::Preparedness, -0.3)
            .with_energy_cost(3.0)
            .with_message(format!("Put {how_many} {what} down in brine"))
    }

    /// How much water a brine takes.
    pub(in crate::analytics) const WHAT_A_BRINE_TAKES: f32 = 1.0;

    /// `Action::Taste`.
    pub(in crate::analytics) fn tasting(&mut self, agent_index: usize, rng: &mut rand::rngs::StdRng) -> ActionResult {
        use crate::world::Position;

        let agent_position = self.population.agents[agent_index].state.position;
        let here = Position::new(agent_position.0, agent_position.1);

        let Some(index) = self.world.node_numbers_on(here).into_iter().find(|&number| {
            let resource = &self.world.resources[number];
            resource.resource_type == crate::world::ResourceType::StrangePlant
                && resource.amount > 0
        }) else {
            return ActionResult::failure("Nothing here to try".to_string());
        };

        let kind = self.world.resources[index].kind;
        let feeds_you = self.world.does_this_one_feed_you(kind);

        self.world.resources[index].harvest(1);
        self.world.did_that_empty_it(index);

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
            // **A poisoning, not a blow.** This came in by `take_damage`,
            // which is the door a blow comes in by, and did everything a blow
            // does: booked as "a blow", so a death of a bad plant was reckoned
            // a death in combat; opened a wound as a blow of that weight
            // would, which could go bad; and was softened by standing near a
            // building, as though walls kept poison out. It is an illness, and
            // it goes straight to the body. See ISSUES_FOUND #309.
            agent.state.lose_health(harm, crate::agents::AgentState::ILLNESS);

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
        turn_now: u32,
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
            .take_a_remedy(&remedy, turn_now)
            .unwrap_or(0.0);

        self.population.agents[agent_index].skills.practise(
            crate::agents::SkillType::Herbalism,
            Self::WHAT_DOSING_SOMEBODY_TEACHES,
            turn_now,
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
