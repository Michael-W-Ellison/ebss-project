//! Visualization Demo Example
//!
//! This example demonstrates all the visualization modes and features
//! available in the EBSS ASCII renderer.
//!
//! Run with: cargo run --example visualization_demo
//!
//! The demo cycles through all visualization modes:
//! - Full Mode: Complete view with world, agents, and statistics
//! - Dashboard Mode: Statistics-focused with trend charts and drive overview
//! - Compact Mode: Single-line status for logging/batch runs
//! - World Focus Mode: Larger map with compact stats
//! - Agent Focus Mode: Detailed individual agent information

use ebss::agents::{Population, AgentConfig};
use ebss::visualization::{AsciiRenderer, RenderMode, RenderConfig};
use std::time::Duration;
use std::thread;

fn main() {
    println!("EBSS Visualization Demo");
    println!("=======================\n");
    println!("This demo will cycle through all visualization modes.");
    println!("Press Ctrl+C to exit at any time.\n");

    // Create a population with some agents
    let mut population = Population::new();

    // Spawn 12 agents with varied positions
    for i in 0..12 {
        population.spawn_agent(AgentConfig::default());

        // Spread agents across the grid
        if let Some(agent) = population.agents.last_mut() {
            agent.state.position = ((i * 7) % 20, (i * 3) % 20, 0);
        }
    }

    // Create a renderer with custom configuration
    let config = RenderConfig {
        width: 80,
        height: 40,
        use_color: true,
        use_unicode: true,
        max_agents_display: 8,
        world_grid_size: 20,
        history_length: 100,
        ..Default::default()
    };

    let mut renderer = AsciiRenderer::with_config(config);

    // Demo sequence
    let modes = [
        (RenderMode::Full, "Full Mode", 50),
        (RenderMode::Dashboard, "Dashboard Mode", 50),
        (RenderMode::WorldFocus, "World Focus Mode", 40),
        (RenderMode::AgentFocus, "Agent Focus Mode", 40),
        (RenderMode::Compact, "Compact Mode", 100),
    ];

    let mut tick = 0u32;
    let mut mode_index = 0;
    let mut ticks_in_mode = 0;

    println!("Starting visualization demo in 2 seconds...\n");
    thread::sleep(Duration::from_secs(2));

    loop {
        let (mode, mode_name, mode_duration) = modes[mode_index];

        // Switch mode at start of each mode's run
        if ticks_in_mode == 0 {
            renderer.set_mode(mode);

            // Log the mode change event
            renderer.log_event(format!("Switched to {} mode", mode_name));
        }

        // Simulate agent activity
        population.tick();

        // Record history for trend tracking
        renderer.record_history(&population, tick);

        // Log periodic events
        if tick % 10 == 0 {
            let alive = population.agents.iter().filter(|a| a.state.health > 0.0).count();
            renderer.log_event(format!("Tick {}: {} agents alive", tick, alive));
        }

        // Render based on current mode
        renderer.render(&population, tick);

        // For compact mode, add a newline periodically
        if mode == RenderMode::Compact && tick % 10 == 0 {
            println!();
        }

        // Progress tracking
        tick += 1;
        ticks_in_mode += 1;

        // Switch to next mode after duration
        if ticks_in_mode >= mode_duration {
            ticks_in_mode = 0;
            mode_index = (mode_index + 1) % modes.len();

            // If we've completed all modes, cycle back
            if mode_index == 0 && tick > 0 {
                println!("\n\n=== Completed one full cycle of all modes ===");
                println!("Total ticks: {}", tick);
                println!("Continuing to cycle...\n");
                thread::sleep(Duration::from_secs(2));
            }
        }

        // Adjust sleep based on mode
        let sleep_ms = match mode {
            RenderMode::Full | RenderMode::Dashboard => 150,
            RenderMode::WorldFocus | RenderMode::AgentFocus => 200,
            RenderMode::Compact => 50,
        };
        thread::sleep(Duration::from_millis(sleep_ms));
    }
}
