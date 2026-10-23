// src/analytics/doing/herding.rs
//! Keeping beasts (#354): a young one raised by hand, fed, milked and plucked,
//! shut in a fold at night, its dung carried out, and one killed when it is
//! wanted.
//!
//! One method per `Action` variant, called from the dispatcher in
//! [`super::execute_action`].

use super::super::Simulation;
use crate::core::DriveType;
use crate::environment::fauna::AnimalSpecies;
use crate::environment::ActionResult;

impl Simulation {
    /// How near a kept beast comes when its keeper wants it: it follows
    /// within this, and is fed, milked or killed from it.
    pub(in crate::analytics) const A_KEPT_BEAST_COMES_TO_HAND: i32 = 3;

    /// How far from a kill a young one of the herd is standing to be left
    /// behind by it, and follow the hunter instead.
    pub(in crate::analytics) const A_YOUNG_ONE_LEFT_BEHIND: i32 = 25;

    /// How tame a young one is the day it is taken: it follows, and will run
    /// off again if nobody feeds it (`the_kept_stay_by_their_keepers`).
    pub(in crate::analytics) const HOW_TAME_A_YOUNG_ONE_TAKEN_IS: f32 = 0.2;

    /// What a day's tending does to a young one's tameness with nothing to
    /// feed it from the hand: led to graze and handled.
    pub(in crate::analytics) const WHAT_TENDING_TAMES: f32 = 0.12;

    /// What a handful of fodder takes off a beast's hunger, as a share of
    /// what it can hold.
    pub(in crate::analytics) const WHAT_A_HANDFUL_FEEDS: f32 = 0.3;

    /// How long after a feed it is before a beast is fed again, in ticks:
    /// half a day.
    pub(in crate::analytics) const FED_TODAY: u32 = crate::environment::seasons::TICKS_PER_DAY / 2;

    /// The wood a fold of brush and poles takes.
    pub(in crate::analytics) const WHAT_A_FOLD_TAKES: u32 = 4;

    /// The most dung carried out of a fold at once.
    pub(in crate::analytics) const AN_ARMFUL_OF_DUNG: u32 = 6;

    /// What a person carries that a beast will eat: whatever grew, and the
    /// leaves before the grain.
    pub(in crate::analytics) fn fodder_in(agent: &crate::agents::Agent) -> Option<String> {
        use crate::world::ItemType;
        let rank = |kind: ItemType| match kind {
            ItemType::Greens => Some(0),
            ItemType::Roots => Some(1),
            ItemType::Food => Some(2),
            ItemType::Legumes => Some(3),
            ItemType::Nuts => Some(4),
            ItemType::Grain => Some(5),
            _ => None,
        };
        agent
            .inventory
            .get_all_items()
            .values()
            .filter(|item| item.quantity > 0)
            .filter(|item| !item.food_data.as_ref().is_some_and(|food| food.is_ruined()))
            .filter_map(|item| {
                let kind = crate::agents::storage_integration::id_to_item_type(&item.item_id)?;
                rank(kind).map(|rank| (rank, item.item_id.clone()))
            })
            .min()
            .map(|(_, id)| id)
    }

    /// How far a beast is from somebody, in the cells either way.
    fn how_far_off(at: (i32, i32), from: (i32, i32, i32)) -> i32 {
        (at.0 - from.0).abs().max((at.1 - from.1).abs())
    }

    /// `Action::Tame`: a kept beast fed from the hand, or - by somebody who
    /// has raised one - a young one caught.
    ///
    /// A young one left standing by a kill follows the hunter home
    /// (`a_young_one_left_behind`) and stays only if it is fed. Fed every
    /// day for a week it is tame for good, and its keeper has found out that
    /// a beast can be kept (`do:Tame`). A tame beast is fed the same way when
    /// the grazing has not filled it.
    pub(in crate::analytics) fn taming(
        &mut self,
        animal_id: &uuid::Uuid,
        food_type: &Option<String>,
        agent_index: usize,
        turn_now: u32,
    ) -> ActionResult {
        let me = self.population.agents[agent_index].id;
        let here = self.population.agents[agent_index].state.position;
        let Some(animal) = self.world.animals.get(animal_id) else {
            return ActionResult::failure("Animal not found".to_string());
        };
        if !animal.is_alive() {
            return ActionResult::failure("Animal is dead".to_string());
        }
        let Some(species) = self.world.animals.get_species(&animal.species_id).cloned() else {
            return ActionResult::failure("Unknown animal species".to_string());
        };
        if !species.can_domesticate {
            return ActionResult::failure(format!("{} will not be kept", species.name));
        }
        if Self::how_far_off(animal.position, here) > Self::A_KEPT_BEAST_COMES_TO_HAND {
            return ActionResult::failure(format!("The {} is too far off", species.name));
        }

        match animal.owner_id {
            Some(keeper) if keeper != me => {
                ActionResult::failure(format!("Somebody else keeps that {}", species.name))
            }
            None => {
                // Catching one is for whoever has raised one before
                if !self.population.agents[agent_index].knows_how_to_do("Tame") {
                    return ActionResult::failure("Nobody here has kept a beast".to_string());
                }
                if animal.is_mature() {
                    return ActionResult::failure(format!("A grown {} will not be taken", species.name));
                }
                let animal = self.world.animals.get_mut(animal_id).expect("looked at above");
                animal.owner_id = Some(me);
                animal.taken_at = turn_now;
                animal.tame_level = animal.tame_level.max(Self::HOW_TAME_A_YOUNG_ONE_TAKEN_IS);
                ActionResult::success()
                    .with_energy_cost(6.0)
                    .with_message(format!("Caught a young {}", species.name))
            }
            Some(_) => {
                if animal.fed_at.is_some_and(|fed| turn_now.saturating_sub(fed) < Self::FED_TODAY) {
                    return ActionResult::failure(format!("The {} has been fed today", species.name));
                }
                let agent = &self.population.agents[agent_index];
                let fodder = food_type
                    .clone()
                    .filter(|food| agent.how_many_i_have(food) > 0)
                    .or_else(|| Self::fodder_in(agent));
                // Nothing in the pack to give it, and a young one is led to
                // graze and handled instead: slower, but it is the handling
                // that tames. Measured, sixteen young taken from kills in
                // forty worlds were never once fed - the plants a keeper
                // carried were eaten by supper - and all sixteen ran off.
                let Some(fodder) = fodder else {
                    if animal.is_domesticated {
                        return ActionResult::failure("Nothing to feed it".to_string());
                    }
                    let animal = self.world.animals.get_mut(animal_id).expect("looked at above");
                    animal.feed(animal.max_hunger * Self::WHAT_A_HANDFUL_FEEDS / 2.0);
                    animal.fed_at = Some(turn_now);
                    animal.tame(Self::WHAT_TENDING_TAMES);
                    let now_tame = animal.is_domesticated;
                    let agent = &mut self.population.agents[agent_index];
                    agent.skills.practise(crate::agents::skills::SkillType::Farming, 2, turn_now);
                    if now_tame {
                        agent.found_out_how_to("do:Tame");
                        return ActionResult::success()
                            .with_energy_cost(3.0)
                            .with_message(format!("The {} is tame and stays", species.name));
                    }
                    return ActionResult::success()
                        .with_energy_cost(3.0)
                        .with_message(format!("Led the {} to graze", species.name));
                };
                let hand = agent
                    .skills
                    .get_skill_if_exists(crate::agents::skills::SkillType::Farming)
                    .map(|skill| skill.level)
                    .unwrap_or(-5);
                let tamer = (0.25 + hand as f32 * 0.02).max(0.1);

                self.population.agents[agent_index].inventory.remove_item(&fodder, 1);
                let animal = self.world.animals.get_mut(animal_id).expect("looked at above");
                animal.feed(animal.max_hunger * Self::WHAT_A_HANDFUL_FEEDS);
                animal.fed_at = Some(turn_now);
                let was_tame = animal.is_domesticated;
                animal.tame(tamer);
                let now_tame = animal.is_domesticated;

                let agent = &mut self.population.agents[agent_index];
                agent.skills.practise(crate::agents::skills::SkillType::Farming, 2, turn_now);
                if now_tame && !was_tame {
                    agent.found_out_how_to("do:Tame");
                    agent.skills.practise(crate::agents::skills::SkillType::Farming, 10, turn_now);
                    return ActionResult::success()
                        .with_drive_change(DriveType::Utility, -0.3)
                        .with_energy_cost(3.0)
                        .with_message(format!("The {} is tame and stays", species.name));
                }
                ActionResult::success()
                    .with_drive_change(DriveType::Utility, -0.1)
                    .with_energy_cost(3.0)
                    .with_message(format!("Fed the {} {}", species.name, fodder))
            }
        }
    }

    /// A young one of the herd left standing near a kill follows the hunter
    /// instead (#354). True if one did.
    ///
    /// The beginning of keeping beasts: nobody sets out to tame anything. A
    /// hunter kills a ewe and the lamb will not leave; somebody feeds it.
    pub(in crate::analytics) fn a_young_one_left_behind(
        &mut self,
        agent_index: usize,
        species: &AnimalSpecies,
        fell_at: (i32, i32),
    ) -> bool {
        if !species.can_domesticate {
            return false;
        }
        let me = self.population.agents[agent_index].id;
        let young = self
            .world
            .animals
            .get_all()
            .iter()
            .filter(|animal| animal.is_alive() && animal.species_id == species.id)
            .filter(|animal| !animal.is_mature() && !animal.is_kept())
            .map(|animal| (Self::how_far_off(animal.position, (fell_at.0, fell_at.1, 0)), animal.id))
            .filter(|(far, _)| *far <= Self::A_YOUNG_ONE_LEFT_BEHIND)
            .min();
        let Some((_, id)) = young else {
            return false;
        };
        let here = self.population.agents[agent_index].state.position;
        let animal = self.world.animals.get_mut(&id).expect("found above");
        animal.owner_id = Some(me);
        animal.taken_at = self.current_turn;
        animal.tame_level = animal.tame_level.max(Self::HOW_TAME_A_YOUNG_ONE_TAKEN_IS);
        animal.position = (here.0, here.1);
        true
    }

    /// `Action::CollectAnimalProduct`: a kept beast milked or plucked by its
    /// keeper, of whatever is ready that the keeper knows to take.
    pub(in crate::analytics) fn collecting_from_a_beast(
        &mut self,
        animal_id: &uuid::Uuid,
        agent_index: usize,
        turn_now: u32,
    ) -> ActionResult {
        let me = self.population.agents[agent_index].id;
        let here = self.population.agents[agent_index].state.position;
        let Some(animal) = self.world.animals.get(animal_id) else {
            return ActionResult::failure("Animal not found".to_string());
        };
        if !animal.is_alive() || !animal.is_domesticated || animal.owner_id != Some(me) {
            return ActionResult::failure("Not a beast of mine".to_string());
        }
        if Self::how_far_off(animal.position, here) > Self::A_KEPT_BEAST_COMES_TO_HAND {
            return ActionResult::failure("It is too far off".to_string());
        }
        let Some(species) = self.world.animals.get_species(&animal.species_id).cloned() else {
            return ActionResult::failure("Unknown animal species".to_string());
        };
        let agent = &self.population.agents[agent_index];
        let ready: Vec<_> = species
            .living_products
            .iter()
            .filter(|product| animal.has_ready(&product.material_id))
            .filter(|product| agent.what_i_found_out().contains(&product.material_id))
            .cloned()
            .collect();
        if ready.is_empty() {
            return ActionResult::failure("Nothing ready to take".to_string());
        }

        let animal = self.world.animals.get_mut(animal_id).expect("looked at above");
        for product in &ready {
            animal.product_timers.insert(product.material_id.clone(), product.production_time);
        }

        // Milk is food and goes off, and gets its clock where it is drawn,
        // as a gathered armful does (#232).
        let now = self.current_turn;
        let mut result = ActionResult::success().with_energy_cost(3.0);
        let mut taken = Vec::new();
        for product in ready {
            let clock = crate::agents::storage_integration::id_to_item_type(&product.material_id)
                .and_then(|kind| self.food_database.create_food_data(&kind, now));
            let mut item = crate::agents::InventoryItem::new_with_weight(
                product.material_id.clone(),
                product.quantity,
                1.0,
            );
            item.food_data = clock;
            self.population.agents[agent_index].inventory.add_item(item);
            taken.push(format!("{} {}", product.quantity, product.material_id));
            result = result.with_item_gained(crate::environment::ItemStack::new(
                product.material_id.clone(),
                product.quantity,
            ));
        }
        self.population.agents[agent_index]
            .skills
            .practise(crate::agents::skills::SkillType::Farming, 2, turn_now);
        result.with_message(format!("Took {} from the {}", taken.join(", "), species.name))
    }

    /// `Action::PutUpFold`: a ring of brush on poles where the keeper sleeps,
    /// to shut the kept beasts in at night (#354).
    pub(in crate::analytics) fn putting_up_a_fold(&mut self, agent_index: usize, turn_now: u32) -> ActionResult {
        let agent = &self.population.agents[agent_index];
        if !agent.knows_how_to_do("PutUpFold") {
            return ActionResult::failure("Nobody here has folded a beast".to_string());
        }
        let here = crate::world::Position::new(agent.state.position.0, agent.state.position.1);
        if self.world.fold_near(&here).is_some() {
            return ActionResult::failure("There is a fold here".to_string());
        }
        if !agent.inventory.has_item("wood", Self::WHAT_A_FOLD_TAKES) {
            return ActionResult::failure(format!("A fold takes {} wood", Self::WHAT_A_FOLD_TAKES));
        }
        let agent = &mut self.population.agents[agent_index];
        agent.inventory.remove_item("wood", Self::WHAT_A_FOLD_TAKES);
        agent.skills.practise(crate::agents::skills::SkillType::Farming, 3, turn_now);
        self.world.folds.push(crate::world::Fold { at: here, put_up: turn_now, dung: 0.0 });
        ActionResult::success()
            .with_energy_cost(8.0)
            .with_message("Put up a fold for the beasts".to_string())
    }

    /// `Action::MuckOut`: the dung trodden into a fold carried out, to burn
    /// where wood is short or to spread on a field (#354).
    pub(in crate::analytics) fn mucking_out(&mut self, agent_index: usize) -> ActionResult {
        let agent = &self.population.agents[agent_index];
        if !agent.knows_how_to_do("MuckOut") {
            return ActionResult::failure("Nobody here has found a use for dung".to_string());
        }
        let here = crate::world::Position::new(agent.state.position.0, agent.state.position.1);
        let Some(which) = self.world.fold_near(&here) else {
            return ActionResult::failure("No fold here".to_string());
        };
        let taken = (self.world.folds[which].dung.floor() as u32).min(Self::AN_ARMFUL_OF_DUNG);
        if taken == 0 {
            return ActionResult::failure("No dung in the fold".to_string());
        }
        self.world.folds[which].dung -= taken as f32;
        self.population.agents[agent_index]
            .inventory
            .add_item(crate::agents::InventoryItem::new_with_weight("dung".to_string(), taken, 1.0));
        ActionResult::success()
            .with_energy_cost(4.0)
            .with_message(format!("Carried {taken} of dung out of the fold"))
    }

    /// `Action::Slaughter`: one of your own kept beasts killed and cut up,
    /// as a hunter's kill is (#354).
    pub(in crate::analytics) fn slaughtering(
        &mut self,
        animal_id: &uuid::Uuid,
        agent_index: usize,
        rng: &mut rand::rngs::StdRng,
        turn_now: u32,
    ) -> ActionResult {
        let me = self.population.agents[agent_index].id;
        let here = self.population.agents[agent_index].state.position;
        if !self.population.agents[agent_index].knows_how_to_do("Slaughter") {
            return ActionResult::failure("Nobody here keeps beasts to kill".to_string());
        }
        let Some(animal) = self.world.animals.get(animal_id) else {
            return ActionResult::failure("Animal not found".to_string());
        };
        if !animal.is_alive() || !animal.is_domesticated || animal.owner_id != Some(me) {
            return ActionResult::failure("Not a beast of mine".to_string());
        }
        if Self::how_far_off(animal.position, here) > Self::A_KEPT_BEAST_COMES_TO_HAND {
            return ActionResult::failure("It is too far off".to_string());
        }
        let Some(species) = self.world.animals.get_species(&animal.species_id).cloned() else {
            return ActionResult::failure("Unknown animal species".to_string());
        };
        let animal = self.world.animals.get_mut(animal_id).expect("looked at above");
        animal.take_damage(animal.max_health);

        let items_gained = self.the_carcass_is_cut_up(agent_index, &species, rng);
        self.population.agents[agent_index]
            .skills
            .practise(crate::agents::skills::SkillType::Farming, 5, turn_now);
        let mut result = ActionResult::success()
            .with_drive_change(DriveType::Hunger, -0.4)
            .with_energy_cost(6.0)
            .with_message(format!("Killed a {} of the herd", species.name));
        for item in items_gained {
            result = result.with_item_gained(item);
        }
        result
    }

    /// A beast that has come down, cut up where it lies: what comes off it,
    /// the meat cut as far as a turn takes it (`the_beast_comes_down`), the
    /// rest into the pack or left on the ground, and the knife one job
    /// further through its life. What came off it, for the result.
    pub(in crate::analytics) fn the_carcass_is_cut_up(
        &mut self,
        agent_index: usize,
        species: &AnimalSpecies,
        rng: &mut rand::rngs::StdRng,
    ) -> Vec<crate::environment::ItemStack> {
        use rand::Rng;

        // What comes off it is as good as the flake that took it off - see
        // the same stamp on the fighting branch.
        let as_good_a_knife = self.population.agents[agent_index]
            .how_well_made_is_what_i_work_this_trade_with(
                crate::agents::skills::SkillType::Leatherworking,
            );

        let mut items_gained = Vec::new();
        for drop in &species.drops {
            if rng.gen_bool(drop.drop_chance as f64) {
                let quantity = rng.gen_range(drop.min_quantity..=drop.max_quantity);
                items_gained.push(match as_good_a_knife {
                    Some(quality) => crate::environment::ItemStack::of_quality(
                        drop.material_id.clone(),
                        quantity,
                        quality,
                    ),
                    None => crate::environment::ItemStack::new(drop.material_id.clone(), quantity),
                });
            }
        }

        let where_it_fell = {
            let at = self.population.agents[agent_index].state.position;
            crate::world::Position::new(at.0, at.1)
        };

        // The meat is cut as far as a turn takes it and the rest left on the
        // bone - see `the_beast_comes_down`.
        self.the_beast_comes_down(agent_index, &mut items_gained, where_it_fell, rng);

        // How much of the rest comes off, which is the waste question and so
        // reads the yield channel: a fine flake leaves less of a hide than a
        // crude one, and a worn flake leaves no more than a fresh one.
        let knife = self.population.agents[agent_index]
            .how_much_my_tools_bring_back(crate::agents::skills::SkillType::Leatherworking);
        let butchered = self.butcher(&items_gained, knife);
        self.into_the_pack_or_on_the_ground(agent_index, butchered, where_it_fell);

        if knife > 1.0 {
            self.population.agents[agent_index]
                .wear_what_i_worked_with(crate::agents::skills::SkillType::Leatherworking);
        }
        items_gained
    }
}
