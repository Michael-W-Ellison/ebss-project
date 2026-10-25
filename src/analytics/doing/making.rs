// src/analytics/doing/making.rs
//! Turning one thing into another, and putting it on.
//!
//! Crafting, building, clothing, fire, what is held in the hand, and the
//! substitution of one part of a making for another.
//!
//! One method per `Action` variant, called from the dispatcher in
//! [`super::execute_action`]. The bodies are as they were when all fifty-two
//! lived in one five-thousand-line `match`; what changed is that a verb can
//! now be found, read and altered without scrolling past the other fifty-one.

use super::super::{determine_placement_approach, Simulation};
use crate::core::DriveType;
use crate::environment::ActionResult;
use crate::world::spatial_planning::SpatialPlanner;
use log::debug;

impl Simulation {
    /// How much of a roof one turn of work puts up.
    ///
    /// `BuildingType::construction_time` is denominated in turns - ninety for
    /// a burrow, which is just under two days on a forty-eight turn day. A
    /// turn is half an hour, and half an hour with a digging stick does not
    /// make a dugout, so this is not one turn: it is a solid stretch of a
    /// morning's work, and a burrow comes to about four turns of it. A tent
    /// is quicker and a longhouse is the work of a season, which is the
    /// ordering the table already has and this leaves alone.
    pub(in crate::analytics) const A_TURN_OF_BUILDING: u32 = 24;

    /// And what that turn teaches the hands that did it.
    pub(in crate::analytics) const WHAT_A_TURN_OF_BUILDING_TEACHES: u32 = 3;

    /// The middle rung of the ladder: a hole under the floor of a finished
    /// roof.
    ///
    /// "Agents should start by carrying some food on them, then by storing
    /// some in their tent, and finally by storing extra in the pit." A pack, a
    /// tent, a pit - three places to go for food instead of one, and the
    /// middle one is the one this model did not have.
    ///
    /// It is a `Pit` rather than a new kind of container on purpose. A burrow
    /// *is* a hole in the ground and a tent has one under it, and everything a
    /// store has to do - keep to its own clock, take a lining, be found by
    /// `nearest_pit_i_remember`, be filled by `putting_food_by` and opened by
    /// `something_out_of_the_store` - is already written once, for pits. A
    /// second container with its own arithmetic would be a second answer to
    /// the same question, which is the defect this project keeps finding.
    ///
    /// What makes it the middle rung is not what it is but where it is: under
    /// the roof somebody sleeps under, rather than wherever the ground
    /// happened to take a hole.
    fn dig_the_store_under_the_roof(
        &mut self,
        under: crate::world::Position,
        agent_index: usize,
        turn_now: u32,
    ) {
        if self.world.pit_at(under).is_some() {
            return;
        }

        // A store dug under the roof the settlement sleeps under is the
        // settlement's, not the digger's - which is the one place in this
        // model where `ToUsAll` is the truthful answer rather than a
        // convenience. See `world::belonging`.
        self.world.pits.push(crate::world::Pit {
            where_it_is: under,
            holds: Vec::new(),
            covered: true,
            dug: turn_now,
            belongs: crate::world::Belongs::ToUsAll,
        });

        // And the man who dug it knows where it is, which is the whole of
        // what `SpatialMemoryType::Storage` is for.
        self.population.agents[agent_index].memory.remember_location(
            crate::core::memory::SpatialMemoryType::Storage,
            (under.x, under.y, 0),
        );
    }

    /// `Action::Build`.
    pub(in crate::analytics) fn building(&mut self, structure_type: &String, position: &(i32, i32, i32), agent_index: usize, turn_now: u32) -> ActionResult {
        use crate::world::{BuildingType, Building, Position, ResourceType};

        // Map structure string to BuildingType
        let building_type = match structure_type.as_str() {
            // What "put up a shelter" means to people who have hides
            // and poles and no quarry. Every house in the list needs
            // stone, the cheapest of them thirty, and a settlement
            // that has never had a single block of it spent an eighth
            // of its life saying so.
            "tent" | "skintent" => BuildingType::SkinTent,
            "burrow" | "dugout" => BuildingType::Burrow,
            "windbreak" => BuildingType::Windbreak,
            "leanto" | "lean-to" => BuildingType::LeanTo,
            "brushhut" | "hut" => BuildingType::BrushHut,
            "shelter" => BuildingType::SkinTent,
            "smallhouse" => BuildingType::SmallHouse,
            "mediumhouse" => BuildingType::MediumHouse,
            "largehouse" => BuildingType::LargeHouse,
            "workshop" => BuildingType::Workshop,
            "storehouse" => BuildingType::Storehouse,
            "farm" => BuildingType::Farm,
            "structure" => BuildingType::SkinTent,
            _ => BuildingType::SkinTent,
        };

        // What this roof takes, and what would answer it.
        //
        // Two ways of asking, and the first one is the one the specification
        // asks for: **a class rather than a name.** A tent wants poles, a
        // flexible covering and cordage, and a people who scraped their hides
        // into leather should be able to roof with the leather - which, asked
        // by name, they could not.
        //
        // It also closes a hole. `SkinTent` has declared since it was written
        // that it wants eight wood and four hides, and two comments in
        // `world::buildings` say so - but the loop below resolves a
        // `ResourceType` to an item name with three arms and `continue`s on
        // everything else, in the checking pass *and* in the consuming pass.
        // So the hides were neither required nor taken: **every tent ever
        // raised in this model was poles and air.** Four of the nine
        // requirements in the building table are for things that match no arm.
        let by_class = crate::environment::tags::what_this_roof_takes(building_type);

        let requirements = building_type.requirements();
        let mut taking: Vec<(&'static str, u32)> = Vec::new();

        // Check if agent has required resources in inventory
        let agent = &self.population.agents[agent_index];
        let mut has_all_resources = true;
        let mut missing_resources = Vec::new();

        // A roof already going up here is carried on with on the strength of
        // what went into it when it was started (#349): asking for its
        // materials again refused every turn of work on a tent whose builder
        // had put them all into it.
        let carrying_on = self.world.buildings.iter().any(|roof| {
            roof.position == Position::new(position.0, position.1)
                && roof.building_type == building_type
                && !roof.is_completed()
        });

        if carrying_on {
            // Nothing more to take
        } else if let Some(wants) = by_class {
            let holding = |what: &str| agent.how_many_i_have(what);

            match crate::environment::tags::what_would_be_used(wants, &holding) {
                Ok(using) => taking = using,
                Err((short_of, how_many)) => {
                    has_all_resources = false;
                    missing_resources.push(format!("{how_many} {}", short_of.called()));
                }
            }
        } else {
            for req in &requirements {
                let item_id = match req.resource_type {
                    ResourceType::Wood => "wood",
                    ResourceType::Stone => "stone",
                    ResourceType::Iron => "iron",
                    ResourceType::Hides => "hides",
                    // Provisioning a work party, presumably; nine buildings
                    // want it and nothing has ever checked or taken it. Left
                    // as it was rather than turned into a hard requirement on
                    // nine buildings in the same change that fixes the tent.
                    _ => continue,
                };

                if let Some(item) = agent.inventory.get_item(item_id) {
                    if item.quantity < req.amount {
                        has_all_resources = false;
                        missing_resources.push(format!("{} {} (have {})", req.amount - item.quantity, item_id, item.quantity));
                    }
                } else {
                    has_all_resources = false;
                    missing_resources.push(format!("{} {}", req.amount, item_id));
                }
            }
        }

        if !has_all_resources {
            return ActionResult::failure(format!(
                "Missing resources for {:?}: {}",
                building_type,
                missing_resources.join(", ")
            ));
        }

        // A roof already going up here is carried on with, not started again.
        //
        // Nothing in the decision layer had ever finished one. `Build` pushed
        // a `new_under_construction` and the only caller of
        // `add_construction_progress` in the whole model is `ConstructionWork`
        // in the parallel world action system, which this layer does not
        // issue. So measured over eight seeded world-years, **every one of
        // the forty-five burrows a settlement dug was still going up when the
        // last of them died**, and the eight roofs that were ever finished
        // were the ones the world was seeded with. `seeking_shelter` asks
        // `is_completed`, so 185,100 turns of `SeekShelter` - 12.2% of every
        // turn anybody ever took - could only ever be answered by a forest
        // tile. The weather took between fifteen and nineteen per cent of
        // everybody who died.
        //
        // The materials are taken off the builder on the turn the site is
        // started, above, which is why this does not go round
        // `execute_construction_work` and its delivery check: nothing
        // delivers to a site in this model, and asking would refuse every
        // turn of work for ever.
        let here = Position::new(position.0, position.1);
        if let Some(going_up) = self
            .world
            .buildings
            .iter_mut()
            .find(|roof| roof.position == here && !roof.is_completed())
        {
            let hand = self.population.agents[agent_index]
                .skills
                .get_skill_if_exists(crate::agents::skills::SkillType::Construction)
                .map(|skill| skill.level)
                .unwrap_or(0);

            let who = self.population.agents[agent_index].id;
            going_up.a_hand_on_it(who);

            let finished = going_up.add_construction_progress(Self::A_TURN_OF_BUILDING, hand);
            let what = going_up.building_type;
            let how_far = going_up.construction_progress();

            // And a man knows where he worked.
            //
            // `SpatialMemoryType::Shelter` has been in the memory since
            // memories were written and **had no writer at all** - the same
            // shape of fault as the pit before the sight pass was taught to
            // notice one, and the reason nobody has ever gone back to a roof
            // he could not see from where he stood. Written here and where a
            // site is started, so a camp's job is a thing its people can
            // remember rather than a thing the world knows.
            self.population.agents[agent_index].memory.remember_location(
                crate::core::memory::SpatialMemoryType::Shelter,
                (here.x, here.y, 0),
            );

            let agent = &mut self.population.agents[agent_index];
            agent.skills.practise(
                crate::agents::skills::SkillType::Construction,
                Self::WHAT_A_TURN_OF_BUILDING_TEACHES,
                turn_now,
            );

            if finished {
                // A store goes under a roof that will stand; brush will not
                if !what.falls_apart() {
                    self.dig_the_store_under_the_roof(here, agent_index, turn_now);
                }
                return ActionResult::success()
                    .with_drive_change(DriveType::Construction, -0.4)
                    .with_energy_cost(20.0)
                    .with_message(format!("Finished the {what:?}"));
            }

            return ActionResult::success()
                .with_drive_change(DriveType::Construction, -0.1)
                .with_energy_cost(20.0)
                .with_message(format!(
                    "Worked on the {what:?} ({:.0}% up)",
                    how_far * 100.0
                ));
        }

        // Brush goes up where its builder stands: it is cover for tonight,
        // not a site chosen for a settlement, and a lean-to against a tree is
        // what a lean-to is (#349). Only another building stands in its way.
        //
        // And it goes up in one go, holding its builder for as many turns as
        // it takes their hands. Put up a turn at a time, four huts in five
        // were started and left - 6,086 started and 867 finished over forty
        // worlds - six poles and a cord apiece, because whatever won the next
        // turn walked off from it.
        if building_type.falls_apart() {
            if self.world.buildings.iter().any(|b| b.position == here) {
                return ActionResult::failure("Something is built here already".to_string());
            }
            let hand = self.population.agents[agent_index]
                .skills
                .get_skill_if_exists(crate::agents::skills::SkillType::Construction)
                .map(|skill| skill.level)
                .unwrap_or(0);
            let agent = &mut self.population.agents[agent_index];
            for (what, how_many) in &taking {
                agent.inventory.remove_item(what, *how_many);
            }
            let mut building = Building::new_under_construction(building_type, here).with_what_was_brought();
            building.now_belongs_to(crate::world::Belongs::To(agent.id));
            building.a_hand_on_it(agent.id);
            let mut turns = 1;
            while !building.add_construction_progress(Self::A_TURN_OF_BUILDING, hand) {
                turns += 1;
            }
            agent.skills.practise(
                crate::agents::skills::SkillType::Construction,
                Self::WHAT_A_TURN_OF_BUILDING_TEACHES * turns,
                turn_now,
            );
            self.world.add_building(building);
            self.population.agents[agent_index].memory.remember_location(
                crate::core::memory::SpatialMemoryType::Shelter,
                (here.x, here.y, 0),
            );
            return ActionResult::success()
                .with_drive_change(DriveType::Construction, -0.4)
                .with_energy_cost(10.0 * turns as f32)
                .taking_periods(turns)
                .with_message(format!("Finished the {building_type:?}"));
        }

        // Use spatial planning to find optimal build location
        let (_criteria, strategy) = determine_placement_approach(building_type);
        let planner = SpatialPlanner::new(&self.world);

        let optimal_pos = planner.find_optimal_location_with_territory(
            building_type,
            *position,
            strategy,
            self.population.agents[agent_index].id,
        );

        // Use optimal position if found, otherwise fall back to agent's position
        let build_tuple_pos = optimal_pos.unwrap_or_else(|| {
            debug!("No optimal position found for {:?}, using agent position", building_type);
            *position
        });

        let build_pos = Position::new(build_tuple_pos.0, build_tuple_pos.1);
        if self.world.is_position_occupied(&build_pos) {
            return ActionResult::failure("No suitable building location found (all positions occupied)".to_string());
        }

        // Remove resources from agent inventory.
        //
        // Whatever the check above settled on is what comes out of the pack -
        // one list, resolved once. Two loops resolving the same requirements
        // independently is how the hides came to be checked in neither and
        // taken in neither.
        let agent = &mut self.population.agents[agent_index];

        if by_class.is_some() {
            for (what, how_many) in &taking {
                agent.inventory.remove_item(what, *how_many);
            }
        } else {
            for req in &requirements {
                let item_id = match req.resource_type {
                    ResourceType::Wood => "wood",
                    ResourceType::Stone => "stone",
                    ResourceType::Iron => "iron",
                    ResourceType::Hides => "hides",
                    _ => continue,
                };

                agent.inventory.remove_item(item_id, req.amount);
            }
        }

        // Create new building (under construction)
        // Whose roof it is. A dwelling is the man's who put it up; a
        // storehouse, a workshop or a shrine is the settlement's, because
        // nobody builds one of those to live in. See `world::belonging` -
        // and note that until this line the owner field was written by
        // nothing at all, so `owns_house` in the goal world-state has always
        // been false for everybody in every world.
        let mut building = Building::new_under_construction(building_type, build_pos).with_what_was_brought();
        building.now_belongs_to(if building_type.is_residential() {
            crate::world::Belongs::To(self.population.agents[agent_index].id)
        } else {
            crate::world::Belongs::ToUsAll
        });

        // Add building to world
        self.world.add_building(building);

        // The man who put the first turn into it knows where it is - see the
        // note on `SpatialMemoryType::Shelter` below.
        self.population.agents[agent_index].memory.remember_location(
            crate::core::memory::SpatialMemoryType::Shelter,
            (build_pos.x, build_pos.y, 0),
        );

        // Emit building started event for timeline
        #[cfg(feature = "gui")]
        {
            use crate::gui::events::{SimulationEvent, SimulationEventType};
            let agent = &self.population.agents[agent_index];
            let event = SimulationEvent::new(
                self.current_turn,
                SimulationEventType::BuildingStarted {
                    building_type,
                    position: build_pos,
                    builder_id: agent.id,
                },
                Some((build_pos.x, build_pos.y)),
            );
            self.population.pending_events.push(event);
        }

        // Grant Construction XP (more XP for larger buildings)
        let construction_xp = match building_type {
            BuildingType::SmallHouse => 5,
            BuildingType::MediumHouse => 10,
            BuildingType::LargeHouse => 15,
            BuildingType::Workshop => 12,
            BuildingType::Storehouse => 8,
            BuildingType::Farm => 10,
            _ => 5,
        };
        let agent = &mut self.population.agents[agent_index];
        agent.skills.practise(crate::agents::skills::SkillType::Construction, construction_xp, turn_now);

        debug!(
            "Agent {} started construction of {:?} at ({}, {})",
            agent.id, building_type, position.0, position.1
        );

        ActionResult::success()
            .with_drive_change(DriveType::Construction, -0.2)
            .with_energy_cost(20.0)
            .with_message(format!("Started building {:?}", building_type))
    }

    /// `Action::Craft`.
    /// The effort of a making an ordinary hand gets through in one planning
    /// period. A spear, a handaxe or a basket is about this; a lashing is
    /// half of it; a wheel was twice it.
    pub(in crate::analytics) const WHAT_A_PERIOD_OF_MAKING_COMES_TO: f32 = 8.0;

    pub(in crate::analytics) fn crafting(&mut self, item_type: &String, agent_index: usize, turn_now: u32) -> ActionResult {
        // The stone-age chain comes first. These steps take named
        // things and turn out named things, so what one step produces
        // the next can pick up; the table below it cannot express that,
        // because its inputs are only ever things dug out of the ground.
        if crate::environment::making::is_made_not_found(item_type) {
            let step = {
                let agent = &self.population.agents[agent_index];
                let holding = |what: &str| agent.how_many_i_have(what);

                // A step nobody has found out is not a step this
                // agent can take, whatever is in his pack.
                if !agent.knows_how_to_make(item_type) {
                    return ActionResult::failure(format!(
                        "Nobody here knows how to make a {}",
                        item_type
                    ));
                }

                match crate::environment::making::every_way_to_make(item_type)
                    .filter(|step| agent.knows_how_to(step))
                    .filter(|step| {
                        step.wants_in_hand.is_none_or(|wanted| {
                            agent.how_many_i_have(wanted) > 0
                        })
                    })
                    .find(|step| step.makings_to_hand(&holding))
                {
                    Some(step) => *step,
                    None => {
                        // Say what is missing rather than that it cannot
                        // be done: the shortfall is the next job.
                        let short = crate::environment::making::every_way_to_make(item_type)
                            .filter(|step| agent.knows_how_to(step))
                            .filter_map(|step| step.short_of(&holding))
                            .min_by_key(|(_, missing)| *missing);

                        return match short {
                            Some((what, how_many)) => ActionResult::failure(format!(
                                "Cannot make {}: short {} {}",
                                item_type, how_many, what
                            )),
                            None => {
                                // Everything is in the pack, so what
                                // is missing is the thing to do it
                                // with.
                                let wanted = crate::environment::making::every_way_to_make(
                                    item_type,
                                )
                                .filter(|step| agent.knows_how_to(step))
                                .find_map(|step| step.wants_in_hand);

                                match wanted {
                                    Some(tool) => ActionResult::failure(format!(
                                        "Cannot make {}: nothing to do it with, wants a {}",
                                        item_type, tool
                                    )),
                                    None => ActionResult::failure(format!(
                                        "Cannot make {}",
                                        item_type
                                    )),
                                }
                            }
                        };
                    }
                }
            };

            if step.over_a_fire {
                let where_he_is = self.population.agents[agent_index].state.position;
                if self
                    .nearest_fire_from(where_he_is, Self::FIRE_REACH, true)
                    .is_none()
                {
                    return ActionResult::failure(format!(
                        "Cannot make {}: no fire burning here",
                        item_type
                    ));
                }
            }

            // What the work is done with is worn by the doing of it,
            // and is not part of what comes out.
            if let Some(wanted) = step.wants_in_hand {
                if let Some(tool) = crate::environment::making::EVERY_TOOL
                    .iter()
                    .find(|tool| tool.called == wanted)
                {
                    self.population.agents[agent_index]
                        .wear_what_i_worked_with(tool.helps);
                }
            }

            let agent = &mut self.population.agents[agent_index];

            // Whether it comes off at all.
            //
            // "Skill level should determine crafting success chance, while
            // tool quality should cap output quality." The success half was
            // built long ago - `Skill::perform_check` - and had exactly one
            // caller, the tailoring branch. Every other making in the model
            // succeeded on the first try whoever attempted it, so a first-day
            // knapper turned out spears as reliably as a lifetime's flintsman
            // and nothing but the finished article was ever at stake.
            //
            // What is in the hand tells here too, and not on the quality: a
            // poor tool is a *dangerous* tool, and `tool_risk_roll_count`
            // rolls the failure and injury checks again for one.
            //
            // And the hand is the hand at *this* thing (#335): "crafting
            // should not be a general skill, but based on the object being
            // crafted." The trade carries some of the way to a new thing;
            // having made it before carries the rest.
            let with_what = agent.how_well_made_is_what_i_work_this_trade_with(step.hands);
            let at_this = agent.skills.skill_at_making(step.makes, step.hands);
            let attempt = at_this.perform_check(with_what);

            // A practised hand is quicker at it and spends less of itself on
            // it. An ordinary hand does a making of ordinary size in a period;
            // a beginner takes two at it, and a big one takes longer still.
            let hand = at_this.hand();
            let periods = (step.effort / Self::WHAT_A_PERIOD_OF_MAKING_COMES_TO / hand)
                .round()
                .max(1.0) as u32;
            let effort = step.effort / hand.sqrt();

            // Cuts and burns, which are the other tax on a bad hand and a bad
            // tool both - where there is anything in the making to cut or
            // burn anybody (`could_this_making_hurt`).
            let hurt_doing_it = attempt
                .injury
                .filter(|_| crate::environment::making::could_this_making_hurt(&step));
            if let Some(hurt) = hurt_doing_it {
                let harm: f32 = match hurt {
                    crate::agents::skills::InjuryType::Small => 2.0,
                    crate::agents::skills::InjuryType::Large => 8.0,
                };
                // Never all the way down - a burn at the fire has never
                // killed anybody in this model and this is not the change
                // that starts it - but named, because a drain that says
                // nothing is a drain booked to whatever spoke last.
                let harm = harm.min((agent.state.health - 1.0).max(0.0));
                agent.state.lose_health(harm, crate::agents::AgentState::A_MISHAP);
            }

            // How good the makings are, read before they are consumed. The
            // worst of them decides: a spear is a shaft, a point and a
            // lashing, and it is only ever as good as the poorest of the
            // three.
            let out_of = step
                .needs
                .iter()
                .filter_map(|(what, _)| {
                    agent.inventory.get_item(what).and_then(|carried| carried.quality)
                })
                .min();

            for (what, how_many) in step.needs {
                agent.inventory.remove_item(what, *how_many);
            }

            if !attempt.success {
                // The makings are spoiled in the trying, and something is
                // learned from having spoiled them - the same bargain the
                // tailoring branch has always struck.
                agent
                    .skills
                    .practise(step.hands, (step.effort / 8.0).round().max(1.0) as u32, turn_now);
                agent.skills.made_one(step.makes);

                return ActionResult::failure(format!(
                    "Spoiled the makings of a {} in the trying",
                    step.makes
                ))
                .spoiled_in_the_making()
                .with_energy_cost(effort)
                .taking_periods(periods);
            }

            // A thing that took more doing is the heavier thing to
            // carry, and a thing made by a better hand is a better
            // thing: it lasts longer and it works better.
            // A worn-through one of the same thing is thrown away
            // rather than stacked with the new one: stacking would
            // hand the fresh tool the broken one's durability.
            if agent
                .inventory
                .get_item(step.makes)
                .is_some_and(|carried| carried.durability_percentage() <= 0.0)
            {
                let had = agent.inventory.count_item(step.makes);
                agent.inventory.remove_item(step.makes, had);
            }

            let made = agent.a_tool_fresh_from_these_hands_out_of(
                step.makes,
                step.how_many,
                step.effort / 4.0,
                out_of,
            );
            if !agent.inventory.add_item(made) {
                debug!(
                    "Agent {} made {} but had nowhere to put it",
                    agent.id, step.makes
                );
            }

            {
                // A spear teaches more than a length of cordage does,
                // in the proportion the two cost to do.
                let learned = (step.effort / 4.0).round().max(1.0) as u32;
                let skill = agent.skills.get_skill_mut(step.hands);
                skill.gain_experience(learned);
                skill.last_used = turn_now;
            }
            agent.skills.made_one(step.makes);

            return ActionResult::success()
                .with_drive_change(DriveType::Utility, -0.2)
                .with_energy_cost(effort)
                .taking_periods(periods)
                .with_message(format!("Made {} {}", step.how_many, step.makes));
        }

        use crate::world::production::{Quality as ProductionQuality, Recipe, ResourceRequirement, ProductionOutput};
        use crate::world::{ItemType, ResourceType};
        use crate::agents::skills::SkillType;

        // Define skill and knowledge-based crafting recipes
        // Format: (recipe, required_skill_level, what has to be known first)
        // Skill levels: -10 to 10, where 0 is untrained adult
        // Knowledge: a making-chain product the agent must know how to make,
        // so these tools wait on the same discoveries as everything else:
        // shaping wood (a sharpened stick), hafting a stone head (a spear),
        // and working metal (a metal axe).
        let skill_gated_recipes: Vec<(Recipe, i32, Option<&str>)> = vec![
            // BEGINNER (skill -10 to 0): Basic wooden tools - wait on shaping wood
            (Recipe {
                name: "Craft Wooden Axe",
                inputs: vec![ResourceRequirement::new(ResourceType::Wood, 3)],
                outputs: vec![ProductionOutput::new(ItemType::WoodenAxe, 1)],
                base_time: 80,
            }, -5, Some("sharpenedstick")),  // Very easy, once wood can be shaped

            (Recipe {
                name: "Craft Wooden Pickaxe",
                inputs: vec![ResourceRequirement::new(ResourceType::Wood, 3)],
                outputs: vec![ProductionOutput::new(ItemType::WoodenPickaxe, 1)],
                base_time: 80,
            }, -5, Some("sharpenedstick")),

            (Recipe {
                name: "Craft Wooden Hammer",
                inputs: vec![ResourceRequirement::new(ResourceType::Wood, 3)],
                outputs: vec![ProductionOutput::new(ItemType::WoodenHammer, 1)],
                base_time: 80,
            }, -5, Some("sharpenedstick")),

            (Recipe {
                name: "Craft Wooden Spear",
                inputs: vec![
                    ResourceRequirement::new(ResourceType::Wood, 2),
                    ResourceRequirement::new(ResourceType::Stone, 1),
                ],
                outputs: vec![ProductionOutput::new(ItemType::WoodenSpear, 1)],
                base_time: 85,
            }, 1, Some("sharpenedstick")),

            // NOVICE (skill 0-3): Stone tools - wait on hafting a stone head
            (Recipe {
                name: "Craft Stone Axe",
                inputs: vec![
                    ResourceRequirement::new(ResourceType::Stone, 2),
                    ResourceRequirement::new(ResourceType::Wood, 1),
                ],
                outputs: vec![ProductionOutput::new(ItemType::StoneAxe, 1)],
                base_time: 90,
            }, 0, Some("spear")),  // Requires basic training + hafting

            (Recipe {
                name: "Craft Stone Pickaxe",
                inputs: vec![
                    ResourceRequirement::new(ResourceType::Stone, 2),
                    ResourceRequirement::new(ResourceType::Wood, 1),
                ],
                outputs: vec![ProductionOutput::new(ItemType::StonePickaxe, 1)],
                base_time: 90,
            }, 0, Some("spear")),

            (Recipe {
                name: "Craft Stone Hammer",
                inputs: vec![
                    ResourceRequirement::new(ResourceType::Stone, 2),
                    ResourceRequirement::new(ResourceType::Wood, 1),
                ],
                outputs: vec![ProductionOutput::new(ItemType::StoneHammer, 1)],
                base_time: 90,
            }, 0, Some("spear")),

            // APPRENTICE (skill 3-5): Iron tools - wait on working metal
            (Recipe {
                name: "Craft Iron Axe",
                inputs: vec![
                    ResourceRequirement::new(ResourceType::Iron, 2),
                    ResourceRequirement::new(ResourceType::Wood, 1),
                ],
                outputs: vec![ProductionOutput::new(ItemType::IronAxe, 1)],
                base_time: 100,
            }, 3, Some("metalaxe")),  // Requires experience + metalworking

            (Recipe {
                name: "Craft Iron Pickaxe",
                inputs: vec![
                    ResourceRequirement::new(ResourceType::Iron, 2),
                    ResourceRequirement::new(ResourceType::Wood, 1),
                ],
                outputs: vec![ProductionOutput::new(ItemType::IronPickaxe, 1)],
                base_time: 100,
            }, 3, Some("metalaxe")),

            (Recipe {
                name: "Craft Iron Hammer",
                inputs: vec![
                    ResourceRequirement::new(ResourceType::Iron, 2),
                    ResourceRequirement::new(ResourceType::Wood, 1),
                ],
                outputs: vec![ProductionOutput::new(ItemType::IronHammer, 1)],
                base_time: 100,
            }, 3, Some("metalaxe")),

            // JOURNEYMAN (skill 5-8): Advanced weapons - wait on working metal
            (Recipe {
                name: "Craft Iron Sword",
                inputs: vec![
                    ResourceRequirement::new(ResourceType::Iron, 3),
                    ResourceRequirement::new(ResourceType::Wood, 1),
                ],
                outputs: vec![ProductionOutput::new(ItemType::IronSword, 1)],
                base_time: 120,
            }, 5, Some("metalaxe")),  // Requires significant experience + metalworking
        ];

        // Get agent's crafting skill level, and what the agent knows how to make
        let agent_skill_level = self.population.agents[agent_index]
            .skills
            .get_skill(SkillType::Crafting)
            .level;
        let knows = |what: &str| self.population.agents[agent_index].knows_how_to_make(what);

        // Filter recipes by skill level AND knowledge - only show recipes agent can craft
        let available_recipes: Vec<(&Recipe, i32, Option<&str>)> = skill_gated_recipes
            .iter()
            .filter(|(_, required_skill, required_tech)| {
                // Check skill requirement
                if agent_skill_level < *required_skill {
                    return false;
                }

                // Check what has to be known first
                if let Some(first) = required_tech {
                    if !knows(first) {
                        return false;
                    }
                }

                true
            })
            .map(|(recipe, skill, tech)| (recipe, *skill, *tech))
            .collect();

        // Try to find a recipe that matches the item type
        let recipe_match = available_recipes.iter().find(|(r, _, _)| {
            r.outputs.iter().any(|output| {
                format!("{:?}", output.item_type).to_lowercase() == item_type.to_lowercase()
            })
        });

        // If no recipe found in available recipes, check if it exists but agent doesn't meet requirements
        if recipe_match.is_none() {
            // Find the recipe in the full list to give a helpful error message
            let full_recipe = skill_gated_recipes.iter().find(|(r, _, _)| {
                r.outputs.iter().any(|output| {
                    format!("{:?}", output.item_type).to_lowercase() == item_type.to_lowercase()
                })
            });

            if let Some((_, required_skill, required_tech)) = full_recipe {
                // Determine what's missing
                let mut reasons = Vec::new();

                if agent_skill_level < *required_skill {
                    reasons.push(format!("insufficient skill (need {}, have {})",
                        required_skill, agent_skill_level));
                }

                if let Some(first) = required_tech {
                    if !knows(first) {
                        reasons.push(format!("does not know how to make a {}", first));
                    }
                }

                return ActionResult::failure(format!(
                    "Cannot craft {}: {}",
                    item_type,
                    reasons.join(", ")
                ));
            } else {
                return ActionResult::failure(format!(
                    "Unknown recipe: {}",
                    item_type
                ));
            }
        }
        let (recipe, _, _) = recipe_match.unwrap();

        // Check if agent has all required materials in inventory
        let agent = &self.population.agents[agent_index];
        let mut has_all_materials = true;
        let mut missing_materials = Vec::new();

        for req in &recipe.inputs {
            let item_id = match req.resource_type {
                ResourceType::Wood => "wood",
                ResourceType::Stone => "stone",
                ResourceType::Iron => "iron",
                ResourceType::Food => "food",
                _ => continue,
            };

            if let Some(item) = agent.inventory.get_item(item_id) {
                if item.quantity < req.amount {
                    has_all_materials = false;
                    missing_materials.push(format!("{} {} (have {})",
                        req.amount - item.quantity, item_id, item.quantity));
                }
            } else {
                has_all_materials = false;
                missing_materials.push(format!("{} {}", req.amount, item_id));
            }
        }

        if !has_all_materials {
            return ActionResult::failure(format!(
                "Missing materials for {}: {}",
                recipe.name,
                missing_materials.join(", ")
            ));
        }

        // Get agent's crafting skill level (-10 to 10)
        let agent = &mut self.population.agents[agent_index];
        let skill_level = agent.skills.get_skill_mut(SkillType::Crafting).level;

        // Convert skill level (-10 to 10) to skill value (0 to 100) for quality calculation
        // -10 -> 0, 0 -> 50, 10 -> 100
        let skill_value = ((skill_level + 10) * 5) as u8;

        // Determine quality based on skill
        let quality = ProductionQuality::from_skill(skill_value);

        // Calculate actual outputs with quality multiplier
        let outputs = recipe.calculate_output(quality);

        // Consume materials from inventory
        for req in &recipe.inputs {
            let item_id = match req.resource_type {
                ResourceType::Wood => "wood",
                ResourceType::Stone => "stone",
                ResourceType::Iron => "iron",
                ResourceType::Food => "food",
                _ => continue,
            };
            agent.inventory.remove_item(item_id, req.amount);
        }

        // Add crafted items to inventory
        for (output_item, quantity) in outputs {
            let item_id = format!("{:?}", output_item).to_lowercase();

            // Create inventory item with appropriate weight
            let item = crate::agents::InventoryItem::new_with_weight(
                item_id.clone(),
                quantity,
                5.0, // Default weight for crafted tools
            );

            if !agent.inventory.add_item(item) {
                debug!(
                    "Agent {} crafted {} but inventory full, item dropped",
                    agent.id, item_id
                );
            }
        }

        // Grant crafting experience
        let experience_gained = match quality {
            ProductionQuality::Poor => 1,
            ProductionQuality::Common => 2,
            ProductionQuality::Good => 3,
            ProductionQuality::Excellent => 4,
            ProductionQuality::Masterwork => 5,
        };

        {
            let skill = agent.skills.get_skill_mut(SkillType::Crafting);
            skill.gain_experience(experience_gained);
            skill.last_used = turn_now;
        }

        debug!(
            "Agent {} crafted {} (quality: {:?}, skill: {}, exp: +{})",
            agent.id, recipe.name, quality, skill_level, experience_gained
        );

        ActionResult::success()
            .with_drive_change(DriveType::Utility, -0.2)
            .with_energy_cost(15.0)
            .with_message(format!("Crafted {} ({:?} quality)", recipe.name, quality))
    }

    /// Wood it takes this person to get a fire going where they stand.
    ///
    /// An unlit hearth already here only wants feeding. Shavings catch where
    /// a log will not, so a hearth laid with tinder under it takes half the
    /// timber to get going. This is what scraping a stick is for - see
    /// `making::SCRAPE_A_STICK`. Shared with the evening at home, which asked
    /// for the full ten while the person stood holding eight and tinder, and
    /// went on gathering what they could not carry (#299).
    pub(in crate::analytics) fn wood_a_fire_here_takes(&self, agent_index: usize) -> u32 {
        let agent = &self.population.agents[agent_index];
        if self
            .nearest_fire_from(agent.state.position, Self::FIRE_REACH, false)
            .is_some()
        {
            Self::FIRE_FUEL_WOOD
        } else if agent.how_many_i_have("tinder") > 0 {
            (Self::FIRE_BUILD_WOOD + Self::FIRE_FUEL_WOOD).div_ceil(2)
        } else {
            Self::FIRE_BUILD_WOOD + Self::FIRE_FUEL_WOOD
        }
    }

    /// `Action::LightFire`.
    pub(in crate::analytics) fn lighting_a_fire(&mut self, agent_index: usize) -> ActionResult {
        // A hearth is worth more than the wood in it, so an unlit fire
        // already standing here is relit rather than rebuilt.
        let agent_pos = self.population.agents[agent_index].state.position;

        let existing = self
            .nearest_fire_from(agent_pos, Self::FIRE_REACH, false)
            .map(|(id, _)| id);

        let wood_needed = self.wood_a_fire_here_takes(agent_index);

        {
            let agent = &self.population.agents[agent_index];
            if agent.fuel_i_carry() < wood_needed {
                return ActionResult::failure(format!(
                    "Not enough wood for a fire: needs {}",
                    wood_needed
                ));
            }
        }

        let builder = self.population.agents[agent_index].id;
        let fire_id = match existing {
            Some(id) => id,
            None => match self.world.build_heat_source(
                crate::environment::HeatSourceType::Campfire,
                agent_pos,
                Some(builder),
            ) {
                Ok(id) => id,
                Err(reason) => {
                    return ActionResult::failure(format!(
                        "Could not build a fire here: {}",
                        reason
                    ))
                }
            },
        };

        // Dried dung first where it is carried, which is good for nothing
        // else, and wood for the rest (#354)
        {
            let agent = &mut self.population.agents[agent_index];
            let dung = if agent.knows_how_to_do("MuckOut") {
                agent.how_many_i_have("dung").min(wood_needed)
            } else {
                0
            };
            if dung > 0 {
                agent.inventory.remove_item("dung", dung);
            }
            if wood_needed > dung {
                agent.inventory.remove_item("wood", wood_needed - dung);
            }
        }

        let _ = self.world.add_fuel_to_heat_source(
            &fire_id,
            "wood".to_string(),
            Self::FIRE_FUEL_WOOD as f32,
        );

        if let Err(reason) = self.world.light_heat_source(&fire_id) {
            return ActionResult::failure(format!("Could not light the fire: {}", reason));
        }

        let agent = &mut self.population.agents[agent_index];
        agent
            .skills
            .gain_experience(crate::agents::SkillType::Cooking, 5);

        debug!("Agent {} lit a fire at {:?}", agent.id, agent_pos);

        ActionResult::success()
            .with_energy_cost(4.0)
            .with_message("Lit a fire".to_string())
    }

    /// `Action::MakeClothing`.
    pub(in crate::analytics) fn making_clothing(&mut self, garment: &String, agent_index: usize, turn_now: u32) -> ActionResult {
        use crate::agents::equipment::garment_recipe;
        use crate::agents::skills::SkillType;

        let recipe = match garment_recipe(garment) {
            Some(recipe) => recipe,
            None => {
                return ActionResult::failure(format!("No such garment: {}", garment))
            }
        };

        let agent = &mut self.population.agents[agent_index];

        if !agent.knows_how_to_make_garment(garment) {
            return ActionResult::failure(format!("Does not know how to make a {}", recipe.name));
        }

        if !agent
            .inventory
            .has_item(recipe.material_item, recipe.material_amount)
        {
            return ActionResult::failure(format!(
                "Not enough {} for a {}: needs {}",
                recipe.material_item, recipe.name, recipe.material_amount
            ));
        }

        // Making a garment is a skill like any other: the same flax in
        // different hands comes out as something that keeps the cold
        // off or as something that falls apart in a week. Quality
        // carries into both warmth and durability.
        //
        // And a beginner does not merely make a worse coat - a good
        // many of their attempts come to nothing at all, with the
        // material spoiled in the trying. `Skill::perform_check` had
        // been built to say exactly this and had no callers anywhere,
        // so every attempt succeeded and a first-day tailor turned out
        // garments as fast as a master. Half of a raw beginner's
        // attempts fail; a master's never do. That is what makes a
        // dedicated tailor quicker as well as better, without anything
        // in the model needing a notion of how long a job takes.
        // What is in the hand tells here too. This passed `None`, so the one
        // branch in the model that already asked whether an attempt came off
        // asked it as though every tailor worked barehanded: a crude awl and
        // a fine one spoiled hides at exactly the same rate.
        let with_what = agent.how_well_made_is_what_i_work_this_trade_with(SkillType::Leatherworking);
        let attempt = agent
            .skills
            .get_skill_mut(SkillType::Leatherworking)
            .perform_check(with_what);

        // Cuts and needle-stabs, which are a beginner's other tax
        if let Some(hurt) = attempt.injury {
            let harm: f32 = match hurt {
                crate::agents::skills::InjuryType::Small => 2.0,
                crate::agents::skills::InjuryType::Large => 8.0,
            };
            let harm = harm.min((agent.state.health - 1.0).max(0.0));
            agent.state.lose_health(harm, crate::agents::AgentState::A_MISHAP);
        }

        if !attempt.success {
            // The material is spoiled in the trying, and something is
            // learned from having spoiled it
            agent
                .inventory
                .remove_item(recipe.material_item, recipe.material_amount);
            agent.skills.practise(SkillType::Leatherworking, 8, turn_now);

            return ActionResult::failure(format!(
                "Spoiled the {} in the making",
                recipe.name
            ))
            .spoiled_in_the_making();
        }

        // What the hand would turn out, capped by what it is working with.
        // "Tool quality should cap output quality" - a master tailor with a
        // crude flake for a knife turns out good work and not fine work.
        //
        // And capped again by the hide itself, which now carries the worth
        // of the flake that skinned it. This is the specification's own
        // example running end to end: "two agents with the same clothing
        // items but of differing quality should have different weather
        // resistances", and what makes them differ is a butchering three
        // actions back.
        let out_of = agent
            .inventory
            .get_item(recipe.material_item)
            .and_then(|carried| carried.quality);

        let quality = {
            let as_far_as_hand_and_tool_go = Self::expected_garment_quality(agent)
                .min(agent.the_best_i_could_turn_out(SkillType::Leatherworking));

            match out_of {
                Some(hide) => as_far_as_hand_and_tool_go.limit_to_material(hide),
                None => as_far_as_hand_and_tool_go,
            }
        };

        let made = match crate::agents::equipment::ClothingTemplate::from_id(
            recipe.id, quality,
        ) {
            Some(made) => made,
            None => {
                return ActionResult::failure(format!("Cannot make a {}", recipe.name))
            }
        };

        agent
            .inventory
            .remove_item(recipe.material_item, recipe.material_amount);

        agent.skills.practise(SkillType::Leatherworking, 25, turn_now);

        // Making a coat and putting it on is one act.
        //
        // Leaving it in the pack to be worn later does not work: an
        // inventory stack carries one quality for the whole stack, so
        // a better second coat merged into the first and was recorded
        // as no better than it. Agents made coat after coat, each an
        // improvement, and wore none of them - over eight thousand
        // turns one settlement made two hundred and eighty garments
        // and put on a hundred and sixty.
        let worn_now = Self::warmth_worn(agent, recipe.slot);
        let put_on = made.cold_insulation() > worn_now;

        if put_on {
            // The coat this replaces is worn out or simply worse, and
            // an agent that kept every one of them ended up carrying
            // twenty cast-offs at two kilos each - a third of what it
            // could carry, in old clothes, instead of food.
            agent.body.unequip(recipe.slot);
            agent.body.equip(made);
        } else {
            let mut folded = crate::agents::InventoryItem::new_with_weight(
                recipe.id.to_string(),
                1,
                2.0,
            );
            folded.quality = Some(quality);
            folded.current_durability = Some(made.durability);
            folded.max_durability = Some(made.max_durability);

            if !agent.inventory.add_item(folded) {
                return ActionResult::failure(format!(
                    "Nowhere to put the {} just made",
                    recipe.name
                ));
            }
        }

        debug!(
            "Agent {} made a {:?} {} (insulation now {:.2})",
            agent.id,
            quality,
            recipe.name,
            agent.body.total_cold_insulation()
        );

        ActionResult::success()
            .with_drive_change(DriveType::Shelter, -0.15)
            .with_energy_cost(8.0)
            .with_message(format!("Made and put on a {:?} {}", quality, recipe.name))
    }

    /// `Action::WearClothing`.
    pub(in crate::analytics) fn wearing_clothing(&mut self, garment: &String, agent_index: usize) -> ActionResult {
        use crate::agents::equipment::{garment_recipe, ClothingTemplate};

        let recipe = match garment_recipe(garment) {
            Some(recipe) => recipe,
            None => {
                return ActionResult::failure(format!("No such garment: {}", garment))
            }
        };

        let agent = &mut self.population.agents[agent_index];

        let carried = match agent.inventory.remove_item(recipe.id, 1) {
            Some(carried) => carried,
            None => {
                return ActionResult::failure(format!("No {} to put on", recipe.name))
            }
        };

        let quality = carried
            .quality
            .unwrap_or(crate::agents::skills::Quality::Common);

        let mut clothing = match ClothingTemplate::from_id(recipe.id, quality) {
            Some(clothing) => clothing,
            None => {
                agent.inventory.add_item(carried);
                return ActionResult::failure(format!("Cannot wear {}", recipe.name));
            }
        };

        // A garment picked back up is as worn as it was when it came off
        if let Some(durability) = carried.current_durability {
            clothing.durability = durability.min(clothing.max_durability);
        }

        // Whatever was in that slot is worse than what is going on
        // over it, and is left behind rather than carried around
        agent.body.unequip(recipe.slot);
        agent.body.equip(clothing);

        debug!(
            "Agent {} put on a {} (insulation now {:.2})",
            agent.id,
            recipe.name,
            agent.body.total_cold_insulation()
        );

        ActionResult::success()
            .with_drive_change(DriveType::Shelter, -0.2)
            .with_energy_cost(1.0)
            .with_message(format!("Put on a {}", recipe.name))
    }

    /// `Action::TrySwapping`.
    pub(in crate::analytics) fn trying_a_swap(&mut self, instead_of_making: &String, instead_of: &String, put_in: &String, agent_index: usize, turn_now: u32) -> ActionResult {
        use crate::environment::making;

        let Some(step) = making::how_to_make(instead_of_making) else {
            return ActionResult::failure("No such job".to_string());
        };

        // The parts have to be to hand: everything the step wants
        // except the one left out, and one of whatever is going in
        // instead.
        {
            let agent = &self.population.agents[agent_index];

            let short = step.needs.iter().any(|(what, how_many)| {
                *what != instead_of.as_str()
                    && agent.how_many_i_have(what) < *how_many
            }) || agent.how_many_i_have(put_in) == 0;

            if short {
                return ActionResult::failure(
                    "Not the makings for that, either way".to_string(),
                );
            }
        }

        let outcome = making::what_comes_of_swapping(
            instead_of_making,
            instead_of,
            put_in,
        );

        // The materials go whether it works or not. That is the whole
        // cost of trying things: a man who puts a lump of iron where
        // the flake goes has spent a stick and a length of cord and
        // has a lump of iron tied to a stick.
        //
        // And he unties the lump. What went in is used up only when the
        // swap worked; a failed one spent the makings and gave the
        // substitute back, where it was destroyed every time (#206).
        let worked = outcome.is_some();
        {
            let agent = &mut self.population.agents[agent_index];
            for (what, how_many) in step.needs {
                if *what == instead_of.as_str() {
                    continue;
                }
                agent.inventory.remove_item(what, *how_many);
            }
            if worked {
                agent.inventory.remove_item(put_in, 1);
            }
        }

        if let Some(swap) = outcome {
            let made = self.population.agents[agent_index]
                .a_tool_fresh_from_these_hands(swap.makes, swap.how_many, 2.0);

            let agent = &mut self.population.agents[agent_index];
            agent.inventory.add_item(made);

            // And he knows how to do it now, which is what makes it a
            // discovery rather than an accident
            agent.found_out_how_to(swap.makes);
            agent.skills.practise(step.hands, 20, turn_now);

            debug!(
                "Agent {} put {put_in} where the {instead_of} goes and got a {}",
                agent.id, swap.makes
            );
        }

        let called = making::what_that_swap_is_called(
            instead_of_making,
            instead_of,
            put_in,
        );
        self.population.agents[agent_index]
            .lessons
            .record_particular(&called, worked);
        self.population.agents[agent_index].tried_this_swap(&called);

        if worked {
            ActionResult::success()
                .with_drive_change(DriveType::Curiosity, -0.5)
                .with_drive_change(DriveType::Utility, -0.3)
                .with_energy_cost(step.effort)
                .with_message(format!("Put {put_in} in and got something new"))
        } else {
            ActionResult::failure(format!(
                "{put_in} where the {instead_of} goes comes to nothing"
            ))
            .with_drive_change(DriveType::Curiosity, -0.3)
            .with_energy_cost(step.effort)
        }
    }

    /// `Action::Equip`.
    pub(in crate::analytics) fn equipping(&mut self, what: &String, agent_index: usize) -> ActionResult {
        // Getting the thing out. It stays in the pack - a hand is a
        // claim on a thing rather than a second place to keep it -
        // and what it buys is `WHAT_A_TOOL_STILL_IN_THE_PACK_IS_WORTH`
        // back on every piece of work done with it.
        let agent = &mut self.population.agents[agent_index];

        if agent.is_in_my_hand(what) {
            return ActionResult::failure(format!("Already holding the {what}"));
        }

        if agent.how_many_i_have(what) == 0 {
            return ActionResult::failure(format!("No {what} to take up"));
        }

        if !agent.take_in_hand(what) {
            return ActionResult::failure("Both hands full".to_string());
        }

        debug!("Agent {} took up a {what}", agent.id);

        ActionResult::success()
            .with_drive_change(DriveType::Utility, -0.1)
            .with_energy_cost(Self::WHAT_GETTING_A_THING_OUT_COSTS)
            .with_message(format!("Took up a {what}"))
    }

    /// `Action::Unequip`.
    pub(in crate::analytics) fn unequipping(&mut self, what: &String, agent_index: usize) -> ActionResult {
        let agent = &mut self.population.agents[agent_index];

        if !agent.put_away(what) {
            return ActionResult::failure(format!("Not holding a {what}"));
        }

        debug!("Agent {} put the {what} away", agent.id);

        ActionResult::success()
            .with_drive_change(DriveType::Utility, -0.1)
            .with_energy_cost(Self::WHAT_GETTING_A_THING_OUT_COSTS)
            .with_message(format!("Put the {what} away"))
    }
}
