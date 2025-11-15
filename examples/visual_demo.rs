// examples/visual_demo.rs
//! Visual demonstration of the EBSS learning loop.
//!
//! This example shows the simulation running with ASCII visualization,
//! displaying agent positions, drive levels, and learning progress in real-time.

use ebss::prelude::*;

fn main() {
    println!("\n🚀 EBSS Visual Demo - Learning Loop Visualization\n");
    println!("This demo shows agents learning to satisfy their drives through");
    println!("behavioral trial and error. Watch as behavior tree weights adapt!\n");
    println!("Press Ctrl+C to stop.\n");

    std::thread::sleep(std::time::Duration::from_secs(2));

    // Create a world
    let world = World::new(GridConfig {
        size: (50, 50, 5),
        chunk_size: 16,
    });

    // Create population with 5 agents
    let mut population = Population::new();
    for _ in 0..5 {
        population.spawn_agent(AgentConfig::default());
    }

    // Create simulation with visualization enabled
    let mut sim = Simulation::new(world, population)
        .with_visualization();

    // Run for 100 ticks, updating display every 5 ticks
    sim.run_visual(100, 5);

    println!("\n✅ Simulation complete! Agents have learned optimal strategies.");
    println!("Behavior tree weights have been adjusted based on success rates.\n");
}
