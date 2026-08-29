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
    /// `Action::Build`.
    pub(in crate::analytics) fn building(&mut self, structure_type: &String, position: &(i32, i32, i32), agent_index: usize, tick_now: u32) -> ActionResult {
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

        // Get resource requirements for this building
        let requirements = building_type.requirements();

        // Check if agent has required resources in inventory
        let agent = &self.population.agents[agent_index];
        let mut has_all_resources = true;
        let mut missing_resources = Vec::new();

        for req in &requirements {
            let item_id = match req.resource_type {
                ResourceType::Wood => "wood",
                ResourceType::Stone => "stone",
                ResourceType::Iron => "iron",
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

        if !has_all_resources {
            return ActionResult::failure(format!(
                "Missing resources for {:?}: {}",
                building_type,
                missing_resources.join(", ")
            ));
        }

        // Use spatial planning to find optimal build location
        let (_criteria, strategy) = determine_placement_approach(building_type);
        let planner = SpatialPlanner::new(&self.world);

        let optimal_pos = planner.find_optimal_location_for_agent(
            building_type,
            *position,  // agent's current position
            strategy
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

        // Remove resources from agent inventory
        let agent = &mut self.population.agents[agent_index];
        for req in &requirements {
            let item_id = match req.resource_type {
                ResourceType::Wood => "wood",
                ResourceType::Stone => "stone",
                ResourceType::Iron => "iron",
                _ => continue,
            };

            agent.inventory.remove_item(item_id, req.amount);
        }

        // Create new building (under construction)
        let building = Building::new_under_construction(building_type, build_pos);

        // Add building to world
        self.world.add_building(building);

        // Emit building started event for timeline
        #[cfg(feature = "gui")]
        {
            use crate::gui::events::{SimulationEvent, SimulationEventType};
            let agent = &self.population.agents[agent_index];
            let event = SimulationEvent::new(
                self.current_tick,
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
        agent.skills.practise(crate::agents::skills::SkillType::Construction, construction_xp, tick_now);

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
    pub(in crate::analytics) fn crafting(&mut self, item_type: &String, agent_index: usize, tick_now: u32) -> ActionResult {
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
            for (what, how_many) in step.needs {
                agent.inventory.remove_item(what, *how_many);
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

            let made = agent.a_tool_fresh_from_these_hands(
                step.makes,
                step.how_many,
                step.effort / 4.0,
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
                skill.last_used = tick_now;
            }

            return ActionResult::success()
                .with_drive_change(DriveType::Utility, -0.2)
                .with_energy_cost(step.effort)
                .with_message(format!("Made {} {}", step.how_many, step.makes));
        }

        use crate::world::production::{Quality as ProductionQuality, Recipe, ResourceRequirement, ProductionOutput};
        use crate::world::{ItemType, ResourceType};
        use crate::agents::skills::SkillType;

        // Define skill and technology-based crafting recipes
        // Format: (recipe, required_skill_level, required_technology)
        // Skill levels: -10 to 10, where 0 is untrained adult
        // Technology: Optional technology ID that must be known
        let skill_gated_recipes: Vec<(Recipe, i32, Option<&str>)> = vec![
            // BEGINNER (skill -10 to 0): Basic wooden tools - requires wooden_tools technology
            (Recipe {
                name: "Craft Wooden Axe",
                inputs: vec![ResourceRequirement::new(ResourceType::Wood, 3)],
                outputs: vec![ProductionOutput::new(ItemType::WoodenAxe, 1)],
                base_time: 80,
            }, -5, Some("wooden_tools")),  // Very easy, needs wooden tools tech

            (Recipe {
                name: "Craft Wooden Pickaxe",
                inputs: vec![ResourceRequirement::new(ResourceType::Wood, 3)],
                outputs: vec![ProductionOutput::new(ItemType::WoodenPickaxe, 1)],
                base_time: 80,
            }, -5, Some("wooden_tools")),

            (Recipe {
                name: "Craft Wooden Hammer",
                inputs: vec![ResourceRequirement::new(ResourceType::Wood, 3)],
                outputs: vec![ProductionOutput::new(ItemType::WoodenHammer, 1)],
                base_time: 80,
            }, -5, Some("wooden_tools")),

            (Recipe {
                name: "Craft Wooden Spear",
                inputs: vec![
                    ResourceRequirement::new(ResourceType::Wood, 2),
                    ResourceRequirement::new(ResourceType::Stone, 1),
                ],
                outputs: vec![ProductionOutput::new(ItemType::WoodenSpear, 1)],
                base_time: 85,
            }, 1, Some("wooden_tools")),

            // NOVICE (skill 0-3): Stone tools - requires stone_tools technology
            (Recipe {
                name: "Craft Stone Axe",
                inputs: vec![
                    ResourceRequirement::new(ResourceType::Stone, 2),
                    ResourceRequirement::new(ResourceType::Wood, 1),
                ],
                outputs: vec![ProductionOutput::new(ItemType::StoneAxe, 1)],
                base_time: 90,
            }, 0, Some("stone_tools")),  // Requires basic training + stone tools tech

            (Recipe {
                name: "Craft Stone Pickaxe",
                inputs: vec![
                    ResourceRequirement::new(ResourceType::Stone, 2),
                    ResourceRequirement::new(ResourceType::Wood, 1),
                ],
                outputs: vec![ProductionOutput::new(ItemType::StonePickaxe, 1)],
                base_time: 90,
            }, 0, Some("stone_tools")),

            (Recipe {
                name: "Craft Stone Hammer",
                inputs: vec![
                    ResourceRequirement::new(ResourceType::Stone, 2),
                    ResourceRequirement::new(ResourceType::Wood, 1),
                ],
                outputs: vec![ProductionOutput::new(ItemType::StoneHammer, 1)],
                base_time: 90,
            }, 0, Some("stone_tools")),

            // APPRENTICE (skill 3-5): Iron tools - requires iron_working technology
            (Recipe {
                name: "Craft Iron Axe",
                inputs: vec![
                    ResourceRequirement::new(ResourceType::Iron, 2),
                    ResourceRequirement::new(ResourceType::Wood, 1),
                ],
                outputs: vec![ProductionOutput::new(ItemType::IronAxe, 1)],
                base_time: 100,
            }, 3, Some("iron_working")),  // Requires experience + iron working tech

            (Recipe {
                name: "Craft Iron Pickaxe",
                inputs: vec![
                    ResourceRequirement::new(ResourceType::Iron, 2),
                    ResourceRequirement::new(ResourceType::Wood, 1),
                ],
                outputs: vec![ProductionOutput::new(ItemType::IronPickaxe, 1)],
                base_time: 100,
            }, 3, Some("iron_working")),

            (Recipe {
                name: "Craft Iron Hammer",
                inputs: vec![
                    ResourceRequirement::new(ResourceType::Iron, 2),
                    ResourceRequirement::new(ResourceType::Wood, 1),
                ],
                outputs: vec![ProductionOutput::new(ItemType::IronHammer, 1)],
                base_time: 100,
            }, 3, Some("iron_working")),

            // JOURNEYMAN (skill 5-8): Advanced weapons - requires iron_working technology
            (Recipe {
                name: "Craft Iron Sword",
                inputs: vec![
                    ResourceRequirement::new(ResourceType::Iron, 3),
                    ResourceRequirement::new(ResourceType::Wood, 1),
                ],
                outputs: vec![ProductionOutput::new(ItemType::IronSword, 1)],
                base_time: 120,
            }, 5, Some("iron_working")),  // Requires significant experience + iron working tech
        ];

        // Get agent's crafting skill level and known technologies
        let (agent_skill_level, agent_known_techs) = {
            let agent = &mut self.population.agents[agent_index];
            let skill = agent.skills.get_skill(SkillType::Crafting).level;
            let techs: std::collections::BTreeSet<String> = agent.technology_knowledge
                .known_technologies.keys().cloned().collect();
            (skill, techs)
        };

        // Filter recipes by skill level AND technology - only show recipes agent can craft
        let available_recipes: Vec<(&Recipe, i32, Option<&str>)> = skill_gated_recipes
            .iter()
            .filter(|(_, required_skill, required_tech)| {
                // Check skill requirement
                if agent_skill_level < *required_skill {
                    return false;
                }

                // Check technology requirement
                if let Some(tech_id) = required_tech {
                    if !agent_known_techs.contains(*tech_id) {
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

                if let Some(tech_id) = required_tech {
                    if !agent_known_techs.contains(*tech_id) {
                        reasons.push(format!("missing technology '{}'", tech_id));
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
            skill.last_used = tick_now;
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

    /// `Action::LightFire`.
    pub(in crate::analytics) fn lighting_a_fire(&mut self, agent_index: usize) -> ActionResult {
        // A hearth is worth more than the wood in it, so an unlit fire
        // already standing here is relit rather than rebuilt.
        let agent_pos = self.population.agents[agent_index].state.position;

        let existing = self
            .nearest_fire_from(agent_pos, Self::FIRE_REACH, false)
            .map(|(id, _)| id);

        // Shavings catch where a log will not, so a hearth laid with
        // tinder under it takes half the timber to get going. This is
        // what scraping a stick is for - see `making::SCRAPE_A_STICK`.
        let has_tinder = self.population.agents[agent_index].how_many_i_have("tinder") > 0;

        let wood_needed = if existing.is_some() {
            Self::FIRE_FUEL_WOOD
        } else if has_tinder {
            (Self::FIRE_BUILD_WOOD + Self::FIRE_FUEL_WOOD).div_ceil(2)
        } else {
            Self::FIRE_BUILD_WOOD + Self::FIRE_FUEL_WOOD
        };

        {
            let agent = &self.population.agents[agent_index];
            if !agent.inventory.has_item("wood", wood_needed) {
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

        self.population.agents[agent_index]
            .inventory
            .remove_item("wood", wood_needed);

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
    pub(in crate::analytics) fn making_clothing(&mut self, garment: &String, agent_index: usize, tick_now: u32) -> ActionResult {
        use crate::agents::equipment::garment_recipe;
        use crate::agents::skills::SkillType;

        let recipe = match garment_recipe(garment) {
            Some(recipe) => recipe,
            None => {
                return ActionResult::failure(format!("No such garment: {}", garment))
            }
        };

        let agent = &mut self.population.agents[agent_index];

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
        let attempt = agent
            .skills
            .get_skill_mut(SkillType::Leatherworking)
            .perform_check(None);

        // Cuts and needle-stabs, which are a beginner's other tax
        if let Some(hurt) = attempt.injury {
            let harm = match hurt {
                crate::agents::skills::InjuryType::Small => 2.0,
                crate::agents::skills::InjuryType::Large => 8.0,
            };
            agent.state.health = (agent.state.health - harm).max(1.0);
        }

        if !attempt.success {
            // The material is spoiled in the trying, and something is
            // learned from having spoiled it
            agent
                .inventory
                .remove_item(recipe.material_item, recipe.material_amount);
            agent.skills.practise(SkillType::Leatherworking, 8, tick_now);

            return ActionResult::failure(format!(
                "Spoiled the {} in the making",
                recipe.name
            ));
        }

        let quality = Self::expected_garment_quality(agent);

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

        agent.skills.practise(SkillType::Leatherworking, 25, tick_now);

        // Making a coat and putting it on is one act.
        //
        // Leaving it in the pack to be worn later does not work: an
        // inventory stack carries one quality for the whole stack, so
        // a better second coat merged into the first and was recorded
        // as no better than it. Agents made coat after coat, each an
        // improvement, and wore none of them - over eight thousand
        // ticks one settlement made two hundred and eighty garments
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
            .unwrap_or(crate::agents::skills::Quality::Basic);

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
    pub(in crate::analytics) fn trying_a_swap(&mut self, instead_of_making: &String, instead_of: &String, put_in: &String, agent_index: usize, tick_now: u32) -> ActionResult {
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
        {
            let agent = &mut self.population.agents[agent_index];
            for (what, how_many) in step.needs {
                if *what == instead_of.as_str() {
                    continue;
                }
                agent.inventory.remove_item(what, *how_many);
            }
            agent.inventory.remove_item(put_in, 1);
        }

        let worked = outcome.is_some();

        if let Some(swap) = outcome {
            let made = self.population.agents[agent_index]
                .a_tool_fresh_from_these_hands(swap.makes, swap.how_many, 2.0);

            let agent = &mut self.population.agents[agent_index];
            agent.inventory.add_item(made);

            // And he knows how to do it now, which is what makes it a
            // discovery rather than an accident
            agent.found_out_how_to(swap.makes);
            agent.skills.practise(step.hands, 20, tick_now);

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
