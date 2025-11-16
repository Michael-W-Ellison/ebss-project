// src/bin/test_simulation.rs
//! Comprehensive test executable for EBSS simulation
//!
//! This executable provides easy testing and troubleshooting of the simulation
//! with full visibility into agent death mechanics (aging, starvation, damage).
//!
//! Usage:
//!   cargo run --bin test_simulation
//!   cargo run --bin test_simulation -- --agents 20 --ticks 5000

use ebss::prelude::*;
use ebss::agents::PopulationConfig;
use std::env;

fn main() {
    // Initialize logging
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    // Parse command line arguments
    let args: Vec<String> = env::args().collect();
    let num_agents = parse_arg(&args, "--agents").unwrap_or(10) as u32;
    let num_ticks = parse_arg(&args, "--ticks").unwrap_or(1000) as u32;
    let report_interval = parse_arg(&args, "--report").unwrap_or(100) as u32;

    println!("╔════════════════════════════════════════════════════════════╗");
    println!("║   EBSS - Emergent Behavior Society Simulator              ║");
    println!("║   Test & Troubleshooting Executable                       ║");
    println!("╚════════════════════════════════════════════════════════════╝");
    println!();
    println!("Configuration:");
    println!("  • Agents: {}", num_agents);
    println!("  • Ticks: {}", num_ticks);
    println!("  • Report Interval: {} ticks", report_interval);
    println!();

    // Create world
    println!("🌍 Creating world...");
    let world = World::new(WorldConfig::default());
    println!("   ✓ World initialized");

    // Create population with configured settings
    println!();
    println!("👥 Spawning population...");
    let config = PopulationConfig::default();
    let mut population = Population::with_config(config);

    for _ in 0..num_agents {
        population.spawn_agent(AgentConfig::default());
    }
    println!("   ✓ {} agents spawned", num_agents);

    // Display initial population statistics
    print_population_status(&population, 0);

    // Create simulation
    println!();
    println!("⚙️  Initializing simulation...");
    let mut sim = Simulation::new(world, population);
    println!("   ✓ Simulation ready");
    println!();
    println!("▶️  Starting simulation for {} ticks...", num_ticks);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    // Run simulation with periodic reporting
    for tick in 0..num_ticks {
        sim.tick();

        // Report at intervals
        if (tick + 1) % report_interval == 0 {
            print_population_status(&sim.population, tick + 1);
            print_death_watch(&sim.population, tick + 1);
        }

        // Check if population died out
        if sim.population.agents.is_empty() {
            println!();
            println!("⚠️  SIMULATION ENDED: Population extinct at tick {}", tick + 1);
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            print_final_statistics(&sim.population, tick + 1);
            return;
        }
    }

    println!();
    println!("✓ Simulation completed successfully!");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    print_final_statistics(&sim.population, num_ticks);
}

/// Parse command line argument
fn parse_arg(args: &[String], flag: &str) -> Option<usize> {
    args.iter()
        .position(|arg| arg == flag)
        .and_then(|i| args.get(i + 1))
        .and_then(|val| val.parse().ok())
}

/// Print current population status
fn print_population_status(population: &Population, tick: u32) {
    let stats = &population.stats;

    println!("📊 Tick {}: Population Status", tick);
    println!("   Population: {} agents", population.agents.len());
    println!("   Life Stages:");
    println!("     • Infants:     {}", stats.infants);
    println!("     • Children:    {}", stats.children);
    println!("     • Adolescents: {}", stats.adolescents);
    println!("     • Adults:      {}", stats.adults);
    println!("     • Elderly:     {}", stats.elderly);
    println!("   Lifetime Stats:");
    println!("     • Total Births: {}", stats.total_births);
    println!("     • Total Deaths: {}", stats.total_deaths);
    println!("     • Abandonments: {}", stats.total_abandonments);
    println!();
}

/// Print agents approaching death (for early warning)
fn print_death_watch(population: &Population, _tick: u32) {
    let mut critical_agents = Vec::new();

    for agent in &population.agents {
        let state = &agent.state;
        let age_percent = (state.age as f32 / state.max_age as f32) * 100.0;

        // Check for critical conditions
        let mut reasons = Vec::new();

        if state.health < 30.0 {
            reasons.push(format!("Low Health: {:.1}", state.health));
        }

        if state.is_starving() {
            let days_without_food = state.ticks_without_food / 1440;
            reasons.push(format!("Starving ({}d)", days_without_food));
        }

        if state.energy < 20.0 {
            reasons.push(format!("Low Energy: {:.1}", state.energy));
        }

        if age_percent > 90.0 {
            reasons.push(format!("Old Age: {:.0}%", age_percent));
        }

        if !reasons.is_empty() {
            critical_agents.push((agent.id, state.life_stage, reasons));
        }
    }

    if !critical_agents.is_empty() {
        println!("⚠️  Critical Agents (Death Watch):");
        for (id, life_stage, reasons) in critical_agents {
            println!("   • {:?} ({:?})", id, life_stage);
            for reason in reasons {
                println!("     - {}", reason);
            }
        }
        println!();
    }
}

/// Print final statistics
fn print_final_statistics(population: &Population, total_ticks: u32) {
    println!();
    println!("📈 Final Statistics");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();
    println!("Simulation Duration: {} ticks ({:.1} sim-days)",
             total_ticks, total_ticks as f32 / 1440.0);
    println!();
    println!("Population Metrics:");
    println!("  Final Population:  {} agents", population.agents.len());
    println!("  Total Births:      {}", population.stats.total_births);
    println!("  Total Deaths:      {}", population.stats.total_deaths);
    println!("  Abandonments:      {}", population.stats.total_abandonments);
    println!();

    if population.stats.total_deaths > 0 {
        let death_rate = (population.stats.total_deaths as f32 / total_ticks as f32) * 1000.0;
        println!("  Death Rate:        {:.2} deaths per 1000 ticks", death_rate);
    }

    if population.stats.total_births > 0 {
        let birth_rate = (population.stats.total_births as f32 / total_ticks as f32) * 1000.0;
        println!("  Birth Rate:        {:.2} births per 1000 ticks", birth_rate);
    }

    println!();

    // Detailed agent breakdown
    if !population.agents.is_empty() {
        println!("Surviving Agents:");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

        for (i, agent) in population.agents.iter().enumerate().take(10) {
            let state = &agent.state;
            let age_percent = (state.age as f32 / state.max_age as f32) * 100.0;

            println!("Agent #{} ({:?}):", i + 1, state.life_stage);
            println!("  Age:     {} / {} ticks ({:.1}%)", state.age, state.max_age, age_percent);
            println!("  Health:  {:.1}/100.0", state.health);
            println!("  Energy:  {:.1}/100.0", state.energy);

            if state.ticks_without_food > 0 {
                let days = state.ticks_without_food / 1440;
                println!("  Hunger:  {} days without food", days);
            } else {
                println!("  Hunger:  Well fed");
            }
            println!();
        }

        if population.agents.len() > 10 {
            println!("... and {} more agents", population.agents.len() - 10);
        }
    }

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
}
