// src/bin/bevy_gui.rs
//! Bevy GUI binary entry point for EBSS.
//!
//! Run with: cargo run --bin ebss_bevy_gui --features bevy_gui

use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use bevy::prelude::{App, default, Window, WindowPlugin};
use bevy::app::PluginGroup;
use bevy::DefaultPlugins;
use bevy_egui::EguiPlugin;

use ebss::prelude::*;
use ebss::world::World as EbssWorld;
use ebss::agents::PopulationConfig;
use ebss::world::TechnologyTree;
use ebss::gui::{
    SimulationCommand as GuiCommand, SimulationSnapshot, SimState as GuiSimState,
    EntitySelection as GuiEntitySelection, SelectedAgentData, SelectedBuildingData,
    SelectedResourceData, TechTreeSnapshot, RelationshipGraphSnapshot,
    simulation_to_snapshot, agent_to_detailed, building_to_detailed, resource_to_detailed,
    tech_tree_to_snapshot, relationship_graph_to_snapshot,
};
use ebss::bevy_gui::{EbssGuiPlugin, SimulationBridge};

fn main() {
    // Initialize logging
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    log::info!("Starting EBSS Bevy GUI...");

    // Create communication channels
    let (command_tx, command_rx): (Sender<GuiCommand>, Receiver<GuiCommand>) = channel();
    let (snapshot_tx, snapshot_rx): (Sender<SimulationSnapshot>, Receiver<SimulationSnapshot>) = channel();

    // Shared state for entity data requests
    let agent_data_request: Arc<Mutex<Option<uuid::Uuid>>> = Arc::new(Mutex::new(None));
    let agent_data_response: Arc<Mutex<Option<SelectedAgentData>>> = Arc::new(Mutex::new(None));
    let building_data_request: Arc<Mutex<Option<ebss::world::Position>>> = Arc::new(Mutex::new(None));
    let building_data_response: Arc<Mutex<Option<SelectedBuildingData>>> = Arc::new(Mutex::new(None));
    let resource_data_request: Arc<Mutex<Option<ebss::world::Position>>> = Arc::new(Mutex::new(None));
    let resource_data_response: Arc<Mutex<Option<SelectedResourceData>>> = Arc::new(Mutex::new(None));
    let tech_tree_request: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
    let tech_tree_response: Arc<Mutex<Option<TechTreeSnapshot>>> = Arc::new(Mutex::new(None));
    let relationship_graph_request: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
    let relationship_graph_response: Arc<Mutex<Option<RelationshipGraphSnapshot>>> = Arc::new(Mutex::new(None));

    // Clone for simulation thread
    let agent_request_clone = Arc::clone(&agent_data_request);
    let agent_response_clone = Arc::clone(&agent_data_response);
    let building_request_clone = Arc::clone(&building_data_request);
    let building_response_clone = Arc::clone(&building_data_response);
    let resource_request_clone = Arc::clone(&resource_data_request);
    let resource_response_clone = Arc::clone(&resource_data_response);
    let tech_tree_request_clone = Arc::clone(&tech_tree_request);
    let tech_tree_response_clone = Arc::clone(&tech_tree_response);
    let relationship_graph_request_clone = Arc::clone(&relationship_graph_request);
    let relationship_graph_response_clone = Arc::clone(&relationship_graph_response);

    // Spawn simulation thread
    thread::spawn(move || {
        run_simulation_thread(
            command_rx,
            snapshot_tx,
            agent_request_clone,
            agent_response_clone,
            building_request_clone,
            building_response_clone,
            resource_request_clone,
            resource_response_clone,
            tech_tree_request_clone,
            tech_tree_response_clone,
            relationship_graph_request_clone,
            relationship_graph_response_clone,
        );
    });

    // Create the simulation bridge resource
    let bridge = SimulationBridge {
        command_tx: Arc::new(Mutex::new(command_tx)),
        snapshot_rx: Arc::new(Mutex::new(snapshot_rx)),
        agent_data_request,
        agent_data_response,
        building_data_request,
        building_data_response,
        resource_data_request,
        resource_data_response,
        tech_tree_request,
        tech_tree_response,
        relationship_graph_request,
        relationship_graph_response,
    };

    // Build and run Bevy app
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "EBSS - Emergent Behavior Society Simulator (Bevy)".into(),
                resolution: (1280.0, 800.0).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(EguiPlugin)
        .insert_resource(bridge)
        .add_plugins(EbssGuiPlugin)
        .run();
}

/// Simulation thread main loop
fn run_simulation_thread(
    command_rx: Receiver<GuiCommand>,
    snapshot_tx: Sender<SimulationSnapshot>,
    agent_data_request: Arc<Mutex<Option<uuid::Uuid>>>,
    agent_data_response: Arc<Mutex<Option<SelectedAgentData>>>,
    building_data_request: Arc<Mutex<Option<ebss::world::Position>>>,
    building_data_response: Arc<Mutex<Option<SelectedBuildingData>>>,
    resource_data_request: Arc<Mutex<Option<ebss::world::Position>>>,
    resource_data_response: Arc<Mutex<Option<SelectedResourceData>>>,
    tech_tree_request: Arc<Mutex<bool>>,
    tech_tree_response: Arc<Mutex<Option<TechTreeSnapshot>>>,
    relationship_graph_request: Arc<Mutex<bool>>,
    relationship_graph_response: Arc<Mutex<Option<RelationshipGraphSnapshot>>>,
) {
    log::info!("Simulation thread starting...");

    // Create world and population
    let world = EbssWorld::new(WorldConfig::default());
    let config = PopulationConfig::default();
    let mut population = Population::with_config(config);

    // Spawn initial agents
    for _ in 0..15 {
        population.spawn_agent(AgentConfig::default());
    }

    let mut simulation = Simulation::new(world, population);

    // Technology tree for the simulation
    let tech_tree = TechnologyTree::new();
    let discovery_history: Vec<(u32, String)> = Vec::new();

    // Simulation state
    let mut state = GuiSimState::Paused;
    let mut speed = 1.0_f32;
    let mut selected = GuiEntitySelection::None;

    // Timing
    let base_tick_duration = Duration::from_millis(16);
    let mut last_tick = Instant::now();
    let mut last_snapshot = Instant::now();
    let snapshot_interval = Duration::from_millis(50);

    log::info!("Simulation initialized with {} agents", simulation.population.agents.len());

    loop {
        // Process commands from GUI
        while let Ok(command) = command_rx.try_recv() {
            match command {
                GuiCommand::Play => {
                    state = GuiSimState::Running;
                    log::info!("Simulation: Play");
                }
                GuiCommand::Pause => {
                    state = GuiSimState::Paused;
                    log::info!("Simulation: Pause");
                }
                GuiCommand::Step => {
                    state = GuiSimState::Stepping;
                    log::info!("Simulation: Step");
                }
                GuiCommand::SetSpeed(new_speed) => {
                    speed = new_speed.clamp(0.1, 10.0);
                    log::info!("Simulation: Speed set to {}x", speed);
                }
                GuiCommand::SelectEntity(entity) => {
                    selected = entity;
                }
                GuiCommand::DeselectAll => {
                    selected = GuiEntitySelection::None;
                }
            }
        }

        // Process agent data requests
        if let Ok(mut request) = agent_data_request.try_lock() {
            if let Some(agent_id) = request.take() {
                if let Some(agent) = simulation.population.agents.iter().find(|a| a.id == agent_id) {
                    let detailed = agent_to_detailed(agent);
                    if let Ok(mut response) = agent_data_response.try_lock() {
                        *response = Some(detailed);
                    }
                }
            }
        }

        // Process building data requests
        if let Ok(mut request) = building_data_request.try_lock() {
            if let Some(pos) = request.take() {
                if let Some(building) = simulation.world.buildings.iter().find(|b| b.position == pos) {
                    let detailed = building_to_detailed(building);
                    if let Ok(mut response) = building_data_response.try_lock() {
                        *response = Some(detailed);
                    }
                }
            }
        }

        // Process resource data requests
        if let Ok(mut request) = resource_data_request.try_lock() {
            if let Some(pos) = request.take() {
                if let Some(resource) = simulation.world.resources.iter().find(|r| r.position == pos) {
                    let detailed = resource_to_detailed(resource);
                    if let Ok(mut response) = resource_data_response.try_lock() {
                        *response = Some(detailed);
                    }
                }
            }
        }

        // Process tech tree data requests
        if let Ok(mut request) = tech_tree_request.try_lock() {
            if *request {
                *request = false;
                let snapshot = tech_tree_to_snapshot(
                    &tech_tree,
                    &simulation.population,
                    &discovery_history,
                );
                if let Ok(mut response) = tech_tree_response.try_lock() {
                    *response = Some(snapshot);
                }
            }
        }

        // Process relationship graph data requests
        if let Ok(mut request) = relationship_graph_request.try_lock() {
            if *request {
                *request = false;
                let snapshot = relationship_graph_to_snapshot(
                    &simulation.population,
                    simulation.current_tick,
                );
                if let Ok(mut response) = relationship_graph_response.try_lock() {
                    *response = Some(snapshot);
                }
            }
        }

        // Run simulation tick if appropriate
        let should_tick = match state {
            GuiSimState::Running => {
                let tick_duration = Duration::from_secs_f32(
                    base_tick_duration.as_secs_f32() / speed
                );
                last_tick.elapsed() >= tick_duration
            }
            GuiSimState::Stepping => true,
            GuiSimState::Paused => false,
        };

        if should_tick {
            simulation.tick();
            last_tick = Instant::now();

            if state == GuiSimState::Stepping {
                state = GuiSimState::Paused;
            }
        }

        // Send snapshot to GUI at regular intervals
        if last_snapshot.elapsed() >= snapshot_interval {
            let snapshot = simulation_to_snapshot(&mut simulation, state, speed, &selected);
            let _ = snapshot_tx.send(snapshot);
            last_snapshot = Instant::now();
        }

        // Small sleep to prevent busy-waiting
        thread::sleep(Duration::from_millis(1));
    }
}
