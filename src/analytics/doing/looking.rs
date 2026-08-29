// src/analytics/doing/looking.rs
//! Attending to a thing.
//!
//! Examining something closely, and the tool verbs of the matrix, which are
//! one agent doing one named thing to one named object.
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
    /// `Action::Examine`.
    pub(in crate::analytics) fn examining(&mut self, what: &String, agent_index: usize, rng: &mut rand::rngs::StdRng, tick_now: u32) -> ActionResult {
        use crate::environment::making;

        if self.population.agents[agent_index].how_many_i_have(what) == 0 {
            return ActionResult::failure(format!("No {what} in hand to look at"));
        }

        // What this thing goes into that nobody here has worked out.
        // Looking closely at a lump of something you are already
        // carrying is the cheapest experiment there is - it costs a
        // turn and no materials - and it is the third road into the
        // chain, beside doing a thing twice to see it again and
        // putting the wrong thing where a part goes.
        if making::is_a_familiar_thing(what) {
            return ActionResult::failure(format!(
                "A {what} is a {what}; there is nothing to see in one"
            ))
            .with_energy_cost(1.0);
        }

        let could_be_for: Vec<&'static str> = making::everything_to_find_out()
            .filter(|step| step.needs.iter().any(|(needs, _)| needs == what))
            .map(|step| step.makes)
            .chain(
                making::every_working_to_find_out()
                    .filter(|working| working.to == *what)
                    .map(|working| working.makes),
            )
            .filter(|makes| {
                !self.population.agents[agent_index]
                    .what_i_found_out()
                    .contains(*makes)
            })
            .collect();

        let agent = &mut self.population.agents[agent_index];
        agent.skills.practise(crate::agents::SkillType::Crafting, 4, tick_now);

        let Some(worth_a_look) = could_be_for.first().copied() else {
            return ActionResult::failure(format!("Nothing new about a {what}"))
                .with_drive_change(DriveType::Curiosity, -0.1)
                .with_energy_cost(1.0);
        };

        // Turning it over in your hands is not the same as knowing.
        // Most of the time it tells you nothing, which is why this
        // does not collapse the chain into an afternoon's inspection.
        let hand = agent.skills.hand_for(crate::agents::SkillType::Crafting);
        let odds = (Self::WHAT_LOOKING_CLOSELY_IS_WORTH * hand).clamp(0.0, 0.5);

        if !rng.gen_bool(odds as f64) {
            return ActionResult::failure(format!("Turned the {what} over, none the wiser"))
                .with_drive_change(DriveType::Curiosity, -0.2)
                .with_energy_cost(1.0);
        }

        agent.found_out_how_to(worth_a_look);

        debug!(
            "Agent {} looked at a {what} and saw what it is for ({worth_a_look})",
            agent.id
        );

        ActionResult::success()
            .with_drive_change(DriveType::Curiosity, -0.5)
            .with_energy_cost(1.0)
            .with_message(format!("Looked at a {what}: it is for a {worth_a_look}"))
    }

    /// `Action::Work`.
    pub(in crate::analytics) fn working(&mut self, verb: &String, to: &String, agent_index: usize, rng: &mut rand::rngs::StdRng, tick_now: u32) -> ActionResult {
        use crate::environment::making;

        let Some(working) = making::how_to_work(verb, to) else {
            return ActionResult::failure(format!("Nothing comes of {verb} a {to}"));
        };

        if self.population.agents[agent_index].how_many_i_have(to) < working.how_much {
            return ActionResult::failure(format!("Not enough {to} to {verb}"));
        }

        // Water has to be carried to where the work is, which is the
        // whole reason a vessel matters
        if working.wants_water > 0.0
            && self.population.agents[agent_index].how_much_water_i_carry()
                < working.wants_water
        {
            return ActionResult::failure(format!("Not enough water to {verb} {to}"));
        }

        if working.over_a_fire {
            let stood = self.population.agents[agent_index].state.position;
            if self
                .nearest_fire_from(stood, Self::FIRE_REACH, true)
                .is_none()
            {
                return ActionResult::failure(format!("No fire here to {verb} {to} over"));
            }
        }

        // What comes off it, and how much of it these hands get. A
        // practised hand wastes less of the core.
        let hand = self.population.agents[agent_index]
            .skills
            .hand_for(working.hands);
        let tool = self.population.agents[agent_index]
            .how_much_my_tools_help(working.hands);

        let worth = working.how_many as f32 * hand.min(2.0) * tool.min(2.0);
        let whole = worth.floor();
        let came_off = (whole as u32)
            + u32::from(rng.gen::<f32>() < worth - whole);
        let came_off = came_off.max(1);

        // What comes out. A bowl is a thing you can put water in and a
        // handful of flour is a meal, and neither is a lump of stuff
        // with a name on it - see `Working::holds` and `Working::feeds`.
        let mut made = match working.holds {
            Some(capacity) => crate::agents::InventoryItem::new_container(
                working.makes.to_string(),
                came_off,
                capacity,
            ),
            None => crate::agents::InventoryItem::new_with_weight(
                working.makes.to_string(),
                came_off,
                1.0,
            ),
        };

        if let Some(as_food) = working.feeds {
            made.food_data = self.food_database.create_food_data(&as_food, tick_now);
        }

        {
            let agent = &mut self.population.agents[agent_index];
            agent.inventory.remove_item(to, working.how_much);

            if working.wants_water > 0.0 {
                agent.draw_from_what_i_carry(working.wants_water);
            }

            agent.inventory.add_item(made);
            agent.skills.practise(working.hands, 12, tick_now);

            // Having done it once he can do it on purpose. For the
            // obvious ones this is a formality; for the rest it is the
            // whole of the discovery - somebody with a scraper in his
            // hand and a fire that will not light finds out what
            // shavings are for by making some.
            agent.found_out_how_to(working.makes);
        }

        // And the edge that did it is the worse for it
        if let Some(broke) = self.population.agents[agent_index]
            .wear_what_i_worked_with(working.hands)
        {
            debug!(
                "Agent {} wore out a {broke}",
                self.population.agents[agent_index].id
            );
        }

        debug!(
            "Agent {} {verb} {} {to} into {came_off} {}",
            self.population.agents[agent_index].id,
            working.how_much,
            working.makes
        );

        ActionResult::success()
            .with_drive_change(DriveType::Utility, -0.25)
            .with_energy_cost(working.effort)
            .with_message(format!("{verb} {to} into {came_off} {}", working.makes))
    }
}
