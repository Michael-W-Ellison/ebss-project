// examples/inspector_demo.rs
//! Demonstration of the inspection and simulation control system.
//!
//! This example shows how to:
//! - Control simulation (pause/play/step)
//! - Inspect agents and their stats
//! - View drive states and urgencies
//! - Select and examine terrain
//! - Display detailed agent information

use ebss::prelude::*;
use ebss::analytics::{SimulationController, Inspector, AgentInspectorData};

fn main() {
    println!("=== EBSS Inspector & Simulation Control Demo ===\n");

    // Create a world and population
    let world = World::new(WorldConfig::default());
    let mut population = Population::new();

    // Spawn some agents with different personalities
    println!("Creating agents with varied personalities...");
    for i in 0..5 {
        let mut config = AgentConfig::default();
        config.random_weights = true;
        population.spawn_agent(config);
        println!("  Agent {} spawned", i + 1);
    }

    // Create simulation controller
    let mut controller = SimulationController::new(world, population);
    let mut inspector = Inspector::new();

    println!("\nSimulation Controller created");
    println!("Initial state: {:?}", controller.state);
    println!("Current tick: {}", controller.current_tick);

    // Display control interface
    println!("\n╔═══════════════════════════════════════════════════════════╗");
    println!("║         SIMULATION CONTROLS                               ║");
    println!("╠═══════════════════════════════════════════════════════════╣");
    println!("║  [SPACE]  - Pause/Resume                                  ║");
    println!("║  [S]      - Step one tick                                 ║");
    println!("║  [+/-]    - Increase/Decrease speed                       ║");
    println!("║  [1-5]    - Select agent 1-5                             ║");
    println!("║  [C]      - Clear selection                              ║");
    println!("║  [I]      - Show inspector data                           ║");
    println!("║  [Q]      - Quit                                         ║");
    println!("╚═══════════════════════════════════════════════════════════╝\n");

    // Demonstration mode - run automatically
    demo_simulation_control(&mut controller);
    demo_agent_inspection(&mut controller, &mut inspector);
    demo_drive_analysis(&mut controller, &mut inspector);
    demo_selection_system(&mut inspector);

    println!("\n=== Demo Complete ===");
    println!("\nKey Features Demonstrated:");
    println!("  ✓ Simulation pause/play/step");
    println!("  ✓ Agent selection and inspection");
    println!("  ✓ Drive state visualization");
    println!("  ✓ Urgency-based drive sorting");
    println!("  ✓ Real-time data caching");
    println!("  ✓ Selection management");

    println!("\n\nIntegration Points:");
    println!("  • Connect to GUI framework (egui, iced, etc.)");
    println!("  • Add mouse/keyboard input handling");
    println!("  • Implement terrain selection and highlighting");
    println!("  • Create data visualization (charts, graphs)");
    println!("  • Add relationship tracking between agents");
}

fn demo_simulation_control(controller: &mut SimulationController) {
    println!("\n╔═══════════════════════════════════════════════════════════╗");
    println!("║  DEMONSTRATION: Simulation Control                        ║");
    println!("╚═══════════════════════════════════════════════════════════╝\n");

    println!("1. Initial State:");
    println!("   State: {:?}", controller.state);
    println!("   Tick: {}", controller.current_tick);
    println!("   Tick Rate: {} ticks/sec", controller.tick_rate);

    println!("\n2. Starting simulation...");
    controller.play();
    println!("   State: {:?}", controller.state);

    println!("\n3. Running 10 ticks...");
    for _ in 0..10 {
        controller.tick_once();
    }
    println!("   Current tick: {}", controller.current_tick);

    println!("\n4. Pausing simulation...");
    controller.pause();
    println!("   State: {:?}", controller.state);

    println!("\n5. Single-stepping 3 ticks...");
    for i in 1..=3 {
        controller.step();
        println!("   Step {}: Tick {}", i, controller.current_tick);
    }

    println!("\n6. Adjusting simulation speed...");
    controller.set_tick_rate(50.0);
    println!("   New tick rate: {} ticks/sec", controller.tick_rate);
}

fn demo_agent_inspection(controller: &mut SimulationController, inspector: &mut Inspector) {
    println!("\n╔═══════════════════════════════════════════════════════════╗");
    println!("║  DEMONSTRATION: Agent Inspection                          ║");
    println!("╚═══════════════════════════════════════════════════════════╝\n");

    let agents = controller.get_population().agents.clone();

    if agents.is_empty() {
        println!("No agents to inspect!");
        return;
    }

    // Inspect first agent
    let agent = &agents[0];
    println!("Inspecting Agent #1:");
    println!("  ID: {}", agent.id);
    println!("  Position: {:?}", agent.state.position);
    println!("  Health: {:.1}/100", agent.state.health);
    println!("  Behavior Trees: {}", agent.behavior_trees.len());

    // Get inspector data
    let agent_data = AgentInspectorData::from_agent(agent);

    println!("\n  Drive Summary:");
    println!("    Total Drives: {}", agent_data.drives.len());
    println!("    Active Drives: {}", agent_data.active_drives().len());

    if let Some(urgent) = agent_data.most_urgent_drive {
        println!("    Most Urgent: {:?}", urgent);
    }

    // Cache the data
    inspector.cache_agent_data(agent.id, agent_data);
    println!("\n  ✓ Agent data cached for quick access");
}

fn demo_drive_analysis(controller: &mut SimulationController, inspector: &mut Inspector) {
    println!("\n╔═══════════════════════════════════════════════════════════╗");
    println!("║  DEMONSTRATION: Drive State Analysis                      ║");
    println!("╚═══════════════════════════════════════════════════════════╝\n");

    let agents = controller.get_population().agents.clone();

    if agents.is_empty() {
        return;
    }

    // Analyze drives for first agent
    let agent = &agents[0];
    let agent_data = AgentInspectorData::from_agent(agent);

    println!("Detailed Drive Analysis for Agent #1:\n");

    // Show all drives sorted by urgency
    let sorted_drives = agent_data.drives_by_urgency();

    println!("╔════════════════╤═══════╤═══════╤════════╤══════════╤════════╗");
    println!("║ Drive          │ Value │ Thresh│ Weight │ Urgency  │ Active ║");
    println!("╠════════════════╪═══════╪═══════╪════════╪══════════╪════════╣");

    for drive in sorted_drives.iter().take(13) {
        let active_mark = if drive.is_active { "✓" } else { " " };
        println!("║ {:14} │ {:5.2} │ {:5.2} │ {:6.2} │ {:8.2} │   {}    ║",
            drive.name,
            drive.value,
            drive.threshold,
            drive.weight,
            drive.urgency,
            active_mark
        );
    }
    println!("╚════════════════╧═══════╧═══════╧════════╧══════════╧════════╝");

    // Show only active drives
    println!("\nActive Drives (Above Threshold):");
    let active = agent_data.active_drives();

    if active.is_empty() {
        println!("  No drives currently active");
    } else {
        for drive in active {
            println!("  • {:?} - Urgency: {:.2} - {}",
                drive.drive_type,
                drive.urgency,
                drive.satisfaction
            );
        }
    }

    // Show drive with highest urgency
    if let Some(most_urgent) = sorted_drives.first() {
        println!("\nMost Urgent Drive:");
        println!("  Drive: {}", most_urgent.name);
        println!("  Urgency: {:.2}", most_urgent.urgency);
        println!("  Satisfaction: {}", most_urgent.satisfaction);
        println!("  Status: {}", if most_urgent.is_active { "ACTIVE" } else { "Inactive" });
    }
}

fn demo_selection_system(inspector: &mut Inspector) {
    println!("\n╔═══════════════════════════════════════════════════════════╗");
    println!("║  DEMONSTRATION: Selection System                          ║");
    println!("╚═══════════════════════════════════════════════════════════╝\n");

    println!("1. No selection:");
    println!("   Current selection: {:?}", inspector.get_selection());

    let test_id = ebss::core::dice::name();

    println!("\n2. Selecting an agent...");
    inspector.select_agent(test_id);
    println!("   Current selection: Agent ({}...)", &test_id.to_string()[0..8]);
    println!("   Is agent selected: {}", inspector.is_agent_selected(test_id));

    println!("\n3. Selecting terrain...");
    let terrain_pos = (10, 64, -5);
    inspector.select_terrain(terrain_pos);
    println!("   Current selection: Terrain {:?}", terrain_pos);

    println!("\n4. Clearing selection...");
    inspector.clear_selection();
    println!("   Current selection: {:?}", inspector.get_selection());

    println!("\n  ✓ Selection system working correctly");
}
