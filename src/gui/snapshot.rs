// src/gui/snapshot.rs
//! Snapshot generation for GUI rendering.

use crate::agents::{Agent, Population};
use crate::analytics::Simulation;
use crate::core::DriveType;
use crate::world::{World, Position, BuildingState, Building, ResourceNode, TechEra, TechnologyTree};
use super::state::*;

/// Generate a world snapshot for GUI rendering
pub fn world_to_snapshot(world: &World) -> WorldSnapshot {
    let width = world.grid.width;
    let height = world.grid.height;

    // Snapshot tiles
    let mut tiles = Vec::with_capacity(width * height);
    for y in 0..height {
        for x in 0..width {
            let pos = Position::new(x as i32, y as i32);
            if let Some(tile) = world.grid.get_tile(&pos) {
                tiles.push(TileSnapshot {
                    x: x as i32,
                    y: y as i32,
                    terrain: tile.terrain.terrain_type,
                    walkable: tile.terrain.is_walkable(),
                });
            }
        }
    }

    // Snapshot resources
    let resources: Vec<ResourceSnapshot> = world.resources.iter().map(|r| {
        ResourceSnapshot {
            position: r.position,
            resource_type: r.resource_type,
            amount: r.amount,
            max_amount: r.max_amount,
        }
    }).collect();

    // Snapshot buildings
    let buildings: Vec<BuildingSnapshot> = world.buildings.iter().map(|b| {
        let (completed, progress) = match &b.state {
            BuildingState::Completed => (true, 1.0),
            BuildingState::UnderConstruction { progress, .. } => (false, *progress as f32 / 100.0),
        };
        BuildingSnapshot {
            position: b.position,
            building_type: b.building_type,
            completed,
            progress,
        }
    }).collect();

    WorldSnapshot {
        width,
        height,
        tiles,
        resources,
        buildings,
        tick: world.tick,
    }
}

/// Generate an agent snapshot for map rendering
pub fn agent_to_snapshot(agent: &Agent) -> AgentSnapshot {
    let most_urgent = agent.drives.most_urgent();

    AgentSnapshot {
        id: agent.id,
        position: agent.state.position,
        health: agent.state.health,
        energy: agent.state.energy,
        life_stage: agent.state.life_stage,
        is_alive: agent.state.is_alive,
        most_urgent_drive: most_urgent.map(|drive| drive.drive_type),
    }
}

/// Generate population snapshot
pub fn population_to_snapshot(population: &Population) -> PopulationSnapshot {
    let agents: Vec<AgentSnapshot> = population.agents.iter()
        .filter(|a| a.state.is_alive)
        .map(agent_to_snapshot)
        .collect();

    let stats = &population.stats;

    // Calculate averages
    let (total_health, total_energy) = population.agents.iter()
        .filter(|a| a.state.is_alive)
        .fold((0.0, 0.0), |(h, e), a| (h + a.state.health, e + a.state.energy));

    let count = agents.len().max(1) as f32;

    PopulationSnapshot {
        agents,
        stats: PopulationStatsSnapshot {
            total_agents: population.agents.len(),
            infants: stats.infants,
            children: stats.children,
            adolescents: stats.adolescents,
            adults: stats.adults,
            elderly: stats.elderly,
            total_births: stats.total_births,
            total_deaths: stats.total_deaths,
            average_health: total_health / count,
            average_energy: total_energy / count,
            average_happiness: stats.average_happiness,
        },
    }
}

/// Generate detailed agent data when selected
pub fn agent_to_detailed(agent: &Agent) -> SelectedAgentData {
    // Drives
    let drives: Vec<DriveData> = DriveType::all().iter().filter_map(|dt| {
        agent.drives.get(*dt).map(|drive| DriveData {
            drive_type: *dt,
            value: drive.value,
            weight: drive.weight,
            urgency: drive.urgency(),
        })
    }).collect();

    // Traits
    let traits: Vec<String> = agent.traits.get_traits().iter()
        .map(|t| format!("{:?}", t))
        .collect();

    // Skills with full data
    let mut skills = std::collections::HashMap::new();
    for skill in agent.skills.get_all_skills().values() {
        skills.insert(
            format!("{:?}", skill.skill_type),
            SkillData {
                name: skill.skill_type.name().to_string(),
                level: skill.level,
                experience: skill.experience,
                category: format!("{:?}", skill.category()),
            },
        );
    }

    // Inventory items
    let inventory: Vec<InventoryItemData> = agent.inventory.get_all_items().values().map(|item| {
        InventoryItemData {
            item_id: item.item_id.clone(),
            quantity: item.quantity,
            quality: item.quality.map(|q| format!("{:?}", q)),
            durability: match (item.current_durability, item.max_durability) {
                (Some(cur), Some(max)) => Some((cur, max)),
                _ => None,
            },
            fill_level: match (item.fill_level, item.max_capacity) {
                (Some(cur), Some(max)) => Some((cur, max)),
                _ => None,
            },
        }
    }).collect();

    // Relationships
    let relationships: Vec<RelationshipData> = agent.relationships.get_all().values().map(|rel| {
        RelationshipData {
            other_agent_id: rel.other_agent,
            relationship_type: format!("{:?}", rel.relationship_type),
            bond_strength: rel.bond_strength,
            total_interactions: rel.total_interactions,
        }
    }).collect();

    // Emotions
    let emotions = EmotionData {
        happiness: agent.emotions.happiness,
        anger: agent.emotions.anger,
        fear: agent.emotions.fear,
        sadness: agent.emotions.sadness,
        curiosity: agent.emotions.curiosity,
    };

    // Goals
    let goals: Vec<GoalData> = agent.goals.goals.iter().map(|goal| {
        GoalData {
            description: format!("{:?}", goal.goal_type),
            priority: goal.priority,
            progress: goal.progress,
            completed: goal.completed,
        }
    }).collect();

    // Current activity from plan
    let current_activity = agent.current_plan.as_ref().and_then(|plan| {
        plan.steps.first().map(|step| format!("{:?}", step.action))
    });

    // Survival status
    let survival_status = SurvivalStatus {
        is_starving: agent.state.is_starving(),
        is_dehydrated: agent.state.is_dehydrated(),
        ticks_without_food: agent.state.ticks_without_food,
        ticks_without_water: agent.state.ticks_without_water,
        is_critical: agent.state.is_survival_critical(),
    };

    SelectedAgentData {
        id: agent.id,
        name: format!("Agent {:?}", agent.id),
        position: agent.state.position,
        health: agent.state.health,
        energy: agent.state.energy,
        age: agent.state.age,
        max_age: agent.state.max_age,
        life_stage: agent.state.life_stage,
        drives,
        traits,
        skills,
        inventory,
        relationships,
        emotions,
        goals,
        current_activity,
        survival_status,
        parent_ids: agent.parent_ids.clone(),
    }
}

/// Generate detailed building data when selected
pub fn building_to_detailed(building: &Building) -> SelectedBuildingData {
    let (completed, progress, resources_needed, worker_ids) = match &building.state {
        BuildingState::Completed => (true, 1.0, Vec::new(), Vec::new()),
        BuildingState::UnderConstruction { progress, resources_delivered, workers } => {
            let requirements = building.building_type.requirements();
            let resources_needed: Vec<(String, u32, u32)> = requirements.iter().map(|req| {
                let delivered = resources_delivered
                    .iter()
                    .filter(|r| r.resource_type == req.resource_type)
                    .map(|r| r.amount)
                    .sum::<u32>();
                (format!("{:?}", req.resource_type), delivered, req.amount)
            }).collect();
            (false, *progress as f32 / 100.0, resources_needed, workers.clone())
        }
    };

    let description = building_description(building.building_type);
    let benefits = building_benefits(building.building_type);

    SelectedBuildingData {
        building_type: building.building_type,
        position: building.position,
        completed,
        progress,
        owner_id: building.owner,
        occupant_ids: building.occupants.clone(),
        resources_needed,
        worker_ids,
        description,
        benefits,
    }
}

/// Generate detailed resource data when selected
pub fn resource_to_detailed(resource: &ResourceNode) -> SelectedResourceData {
    let percentage = resource.percentage_remaining();
    let description = resource_description(resource.resource_type);
    let uses = resource_uses(resource.resource_type);

    SelectedResourceData {
        resource_type: resource.resource_type,
        position: resource.position,
        amount: resource.amount,
        max_amount: resource.max_amount,
        percentage,
        is_depleted: resource.is_depleted(),
        description,
        uses,
    }
}

/// Generate complete simulation snapshot
pub fn simulation_to_snapshot(
    simulation: &Simulation,
    state: SimState,
    speed: f32,
    selected: &EntitySelection,
) -> SimulationSnapshot {
    SimulationSnapshot {
        tick: simulation.world.tick,
        state,
        speed,
        world: world_to_snapshot(&simulation.world),
        population: population_to_snapshot(&simulation.population),
        selected: selected.clone(),
    }
}

fn building_description(building_type: crate::world::BuildingType) -> String {
    use crate::world::BuildingType;
    match building_type {
        // Shelter
        BuildingType::Longhouse => "Shared community housing for multiple agents.".to_string(),
        BuildingType::UpgradedLonghouse => "Improved longhouse with better amenities.".to_string(),
        BuildingType::SmallHouse => "Personal dwelling for 1-2 people.".to_string(),
        BuildingType::MediumHouse => "Family home with room for 4 occupants.".to_string(),
        BuildingType::LargeHouse => "Spacious multi-room home for larger families.".to_string(),
        BuildingType::Manor => "Luxury estate with premium living conditions.".to_string(),

        // Civic
        BuildingType::TownCenter => "Administrative center for community governance.".to_string(),
        BuildingType::TownStorage => "Large community storage facility.".to_string(),
        BuildingType::GuardPost => "Security outpost for community defense.".to_string(),

        // Production
        BuildingType::Workshop => "Workspace for crafting tools and items.".to_string(),
        BuildingType::Forge => "Basic metalworking facility.".to_string(),
        BuildingType::Smithy => "Advanced forge for metalworking and tool making.".to_string(),
        BuildingType::Bakery => "Food processing for baked goods.".to_string(),
        BuildingType::WeaverHut => "Textile production facility.".to_string(),
        BuildingType::PotteryKiln => "Kiln for firing pottery and ceramics.".to_string(),
        BuildingType::Tannery => "Leather working facility.".to_string(),
        BuildingType::Mill => "Grain processing into flour.".to_string(),

        // Resource
        BuildingType::Storehouse => "Basic storage for resources.".to_string(),
        BuildingType::Farm => "Agricultural land for growing crops.".to_string(),
        BuildingType::AnimalPen => "Enclosure for domesticated animals.".to_string(),

        _ => "A structure serving various purposes.".to_string(),
    }
}

fn building_benefits(building_type: crate::world::BuildingType) -> Vec<String> {
    use crate::world::BuildingType;
    match building_type {
        BuildingType::Longhouse | BuildingType::SmallHouse |
        BuildingType::MediumHouse | BuildingType::LargeHouse => vec![
            "Provides shelter from weather".to_string(),
            "Increases safety".to_string(),
            "Allows comfortable rest".to_string(),
        ],
        BuildingType::Manor => vec![
            "Premium shelter and comfort".to_string(),
            "High safety rating".to_string(),
            "Status symbol".to_string(),
        ],
        BuildingType::Workshop => vec![
            "Enables advanced crafting".to_string(),
            "Improves crafting quality".to_string(),
            "Required for complex items".to_string(),
        ],
        BuildingType::Smithy | BuildingType::Forge => vec![
            "Metal tool production".to_string(),
            "Weapon crafting".to_string(),
            "Armor production".to_string(),
        ],
        BuildingType::Farm => vec![
            "Produces renewable food".to_string(),
            "Reduces foraging needs".to_string(),
            "Supports larger population".to_string(),
        ],
        BuildingType::Storehouse | BuildingType::TownStorage => vec![
            "Stores resources safely".to_string(),
            "Protects items from decay".to_string(),
            "Increases storage capacity".to_string(),
        ],
        BuildingType::GuardPost => vec![
            "Improves community safety".to_string(),
            "Early threat detection".to_string(),
            "Deters predators".to_string(),
        ],
        _ => vec!["Various benefits".to_string()],
    }
}

fn resource_description(resource_type: crate::world::ResourceType) -> String {
    use crate::world::ResourceType;
    match resource_type {
        // Basic resources
        ResourceType::Wood => "Timber from trees, essential for construction and fuel.".to_string(),
        ResourceType::Stone => "Rock and minerals for building and tools.".to_string(),
        ResourceType::Iron => "Metal ore that can be smelted into iron tools.".to_string(),
        ResourceType::Food => "Edible plants and berries for sustenance.".to_string(),
        ResourceType::Water => "Fresh water source for drinking.".to_string(),

        // Agricultural
        ResourceType::Grain => "Cereal crops for flour, bread, and beer.".to_string(),
        ResourceType::Flax => "Plant fiber for linen textiles and rope.".to_string(),
        ResourceType::Herbs => "Medicinal and culinary plants.".to_string(),
        ResourceType::Cotton => "Soft fiber for clothing and textiles.".to_string(),

        // Animal
        ResourceType::Hides => "Raw animal skins for leather production.".to_string(),
        ResourceType::Wool => "Fiber from sheep for cloth production.".to_string(),
        ResourceType::Meat => "Butchered animal meat for food.".to_string(),
        ResourceType::Milk => "Fresh milk for cheese and butter.".to_string(),
        ResourceType::Fish => "Aquatic food source from rivers and lakes.".to_string(),
        ResourceType::Honey => "Sweet substance from beehives.".to_string(),

        // Mineral
        ResourceType::Clay => "Moldable earth for pottery and bricks.".to_string(),
        ResourceType::Sand => "Silica particles for glass making.".to_string(),
        ResourceType::Coal => "Combustible mineral for fuel and smelting.".to_string(),

        // Processed
        ResourceType::Flour => "Ground grain for baking.".to_string(),
        ResourceType::Leather => "Processed hides for crafting.".to_string(),
        ResourceType::Cloth => "Woven fabric for clothing.".to_string(),
        ResourceType::Glass => "Molten sand formed into glass.".to_string(),
        ResourceType::Bricks => "Fired clay for construction.".to_string(),
        ResourceType::Charcoal => "Processed wood for high-heat fuel.".to_string(),
        ResourceType::Rope => "Twisted fiber for binding and construction.".to_string(),

        _ => "A natural resource with various uses.".to_string(),
    }
}

fn resource_uses(resource_type: crate::world::ResourceType) -> Vec<String> {
    use crate::world::ResourceType;
    match resource_type {
        ResourceType::Wood => vec![
            "Building construction".to_string(),
            "Tool handles".to_string(),
            "Fuel for fires".to_string(),
            "Furniture crafting".to_string(),
        ],
        ResourceType::Stone => vec![
            "Building foundations".to_string(),
            "Stone tools".to_string(),
            "Walls and fortifications".to_string(),
        ],
        ResourceType::Iron => vec![
            "Metal tools".to_string(),
            "Weapons and armor".to_string(),
            "Construction materials".to_string(),
        ],
        ResourceType::Food => vec![
            "Eating raw".to_string(),
            "Cooking meals".to_string(),
            "Preserving for storage".to_string(),
        ],
        ResourceType::Water => vec![
            "Drinking".to_string(),
            "Cooking".to_string(),
            "Irrigation".to_string(),
        ],
        ResourceType::Grain => vec![
            "Grinding into flour".to_string(),
            "Brewing beer".to_string(),
            "Animal feed".to_string(),
        ],
        ResourceType::Herbs => vec![
            "Medicine".to_string(),
            "Seasoning food".to_string(),
            "Remedies".to_string(),
        ],
        ResourceType::Hides => vec![
            "Tanning into leather".to_string(),
            "Fur for warmth".to_string(),
        ],
        ResourceType::Clay => vec![
            "Pottery".to_string(),
            "Brick making".to_string(),
            "Building material".to_string(),
        ],
        ResourceType::Coal => vec![
            "Smelting fuel".to_string(),
            "High-heat forge work".to_string(),
        ],
        _ => vec!["Various uses".to_string()],
    }
}

/// Generate technology tree snapshot for GUI
pub fn tech_tree_to_snapshot(
    tech_tree: &TechnologyTree,
    population: &Population,
    discovery_history: &[(u32, String)],
) -> TechTreeSnapshot {
    let mut nodes = Vec::new();
    let mut total_discovered = 0;
    let mut highest_era = TechEra::StoneAge;

    for tech in tech_tree.all() {
        let era_index = match tech.era {
            TechEra::StoneAge => 0,
            TechEra::CopperAge => 1,
            TechEra::BronzeAge => 2,
            TechEra::IronAge => 3,
            TechEra::Medieval => 4,
        };

        // Count agents who know this tech (using technology_knowledge from environment module)
        let agents_with_knowledge = population.agents.iter()
            .filter(|a| a.state.is_alive && a.technology_knowledge.known_technologies.contains_key(tech.id))
            .count();

        // Check if any agent can discover this (has all prerequisites)
        let can_be_discovered = population.agents.iter()
            .filter(|a| a.state.is_alive)
            .any(|a| {
                // Check if agent has all prerequisites
                let has_all_prereqs = tech.prerequisites.iter()
                    .all(|prereq| a.technology_knowledge.known_technologies.contains_key(*prereq));
                has_all_prereqs && !a.technology_knowledge.known_technologies.contains_key(tech.id)
            });

        // Determine status
        let any_agent_knows = agents_with_knowledge > 0;

        let status = if any_agent_knows {
            total_discovered += 1;
            if tech.era > highest_era {
                highest_era = tech.era;
            }
            TechStatus::Discovered
        } else if can_be_discovered {
            TechStatus::Discoverable
        } else {
            TechStatus::Unknown
        };

        // Find first discoverer and tick from history
        let (first_discoverer, discovery_tick) = discovery_history.iter()
            .find(|(_, id)| id == tech.id)
            .map(|(tick, _)| {
                let discoverer = population.agents.iter()
                    .find(|a| a.technology_knowledge.known_technologies.contains_key(tech.id))
                    .map(|a| a.id);
                (discoverer, Some(*tick))
            })
            .unwrap_or((None, None));

        // Build unlocks list
        let unlocks: Vec<String> = tech.unlocks_recipes.iter()
            .map(|item| format!("{:?}", item))
            .collect();

        nodes.push(TechNodeData {
            id: tech.id.to_string(),
            name: tech.name.to_string(),
            description: tech.description.to_string(),
            era: tech.era.name().to_string(),
            era_index,
            status,
            discovery_progress: 0, // Progress tracking not used in current implementation
            agents_with_knowledge,
            prerequisites: tech.prerequisites.iter().map(|s| s.to_string()).collect(),
            unlocks,
            first_discoverer,
            discovery_tick,
        });
    }

    TechTreeSnapshot {
        nodes,
        current_era: highest_era.name().to_string(),
        total_discovered,
        total_technologies: tech_tree.all().len(),
        discovery_history: discovery_history.to_vec(),
    }
}
