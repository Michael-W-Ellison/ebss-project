// src/gui/snapshot.rs
//! Snapshot generation for GUI rendering.

use crate::agents::{Agent, Population};
use crate::analytics::Simulation;
use crate::core::DriveType;
use crate::world::{World, Position, BuildingState};
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
    let drives: Vec<DriveData> = DriveType::all().iter().filter_map(|dt| {
        agent.drives.get(*dt).map(|drive| DriveData {
            drive_type: *dt,
            value: drive.value,
            weight: drive.weight,
            urgency: drive.urgency(),
        })
    }).collect();

    let traits: Vec<String> = agent.traits.get_traits().iter()
        .map(|t| format!("{:?}", t))
        .collect();

    let mut skills = std::collections::HashMap::new();
    for skill in agent.skills.get_all_skills().values() {
        skills.insert(format!("{:?}", skill.skill_type), skill.level);
    }

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
        inventory_count: agent.inventory.get_all_items().len(),
        relationship_count: agent.relationships.get_all().len(),
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
