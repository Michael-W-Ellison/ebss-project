// examples/phase4_analytics.rs
//! Comprehensive example demonstrating Phase 4 analytics features.
//!
//! This example shows:
//! - Data logging and metrics tracking
//! - Emergence detection
//! - Performance monitoring
//! - Data export for visualization
//!
//! Run with: cargo run --example phase4_analytics

use ebss::prelude::*;
use ebss::agents::{Population, PopulationConfig, AgentConfig};
use ebss::analytics::{
    SimulationMetrics, EmergenceDetector, DataExporter, PerformanceMonitor, ExportFormat,
};

fn main() {
    println!("=== EBSS Phase 4: Analytics and Emergence Detection ===\n");

    // Initialize systems
    let mut population = Population::new();
    population.config = PopulationConfig {
        abandonment_happiness_threshold: -0.3,
        abandonment_unhappy_duration: 1000,
        abandonment_probability: 0.01,
        ..PopulationConfig::default()
    };

    let mut metrics = SimulationMetrics::new(10, 500); // Sample every 10 ticks, keep 500
    let mut emergence = EmergenceDetector::new();
    let mut performance = PerformanceMonitor::new(1000);

    println!("Spawning initial population of 10 agents...");
    for _ in 0..10 {
        let config = AgentConfig::default();
        population.spawn_agent(config);
    }

    println!("Running simulation for 1000 ticks...\n");

    // Main simulation loop
    for tick in 0..1000 {
        performance.start_tick();

        // Process population
        let start = performance.start_operation("population_tick");
        population.tick();
        performance.end_operation("population_tick", start);

        // Process reproduction
        let start = performance.start_operation("reproduction");
        population.process_reproduction();
        performance.end_operation("reproduction", start);

        // Process abandonments
        let start = performance.start_operation("abandonments");
        population.process_abandonments();
        performance.end_operation("abandonments", start);

        // Record metrics
        metrics.record_if_time(tick, &population);

        // Detect emergence patterns every 50 ticks
        if tick % 50 == 0 && tick > 0 {
            emergence.detect_patterns(&metrics, tick);
        }

        performance.end_tick(tick, population.agents.len());

        // Progress indicator
        if tick % 100 == 0 {
            println!(
                "Tick {:4}: Population {} | Avg Happiness {:.2} | TPS {:.1}",
                tick,
                population.agents.len(),
                population.stats.average_happiness,
                performance.snapshots.last().map(|s| s.ticks_per_second).unwrap_or(0.0)
            );
        }
    }

    println!("\n=== Simulation Complete ===\n");

    // Display summary
    let summary = metrics.summary();
    println!("Summary:");
    println!("  Total Ticks: {}", summary.total_ticks);
    println!("  Initial Population: {}", summary.initial_population);
    println!("  Final Population: {}", summary.final_population);
    println!("  Population Change: {:+}", summary.population_change);
    println!("  Peak Population: {}", summary.peak_population);
    println!("  Average Happiness: {:.2}", summary.average_happiness);
    println!("  Total Relationships: {}", summary.total_relationships);
    if let Some(trait_item) = summary.most_common_trait {
        println!("  Most Common Trait: {:?}", trait_item);
    }

    // Display performance summary
    let perf_summary = performance.summary();
    println!("\nPerformance:");
    println!("  Average TPS: {:.1}", perf_summary.average_ticks_per_second);
    println!("  Peak TPS: {:.1}", perf_summary.peak_ticks_per_second);
    println!("  Min TPS: {:.1}", perf_summary.min_ticks_per_second);
    println!("  Total Operations: {}", perf_summary.total_operations);
    println!("  Total Time: {:.2}s", perf_summary.total_time_seconds);

    // Display slowest operations
    println!("\nSlowest Operations:");
    for (name, metrics) in performance.slowest_operations(3) {
        println!(
            "  {}: avg {:.2}ms ({} calls)",
            name,
            metrics.average_duration_micros / 1000.0,
            metrics.total_calls
        );
    }

    // Display emergent patterns
    println!("\nEmergent Patterns Detected: {}", emergence.detected_patterns.len());
    for (i, pattern) in emergence.most_severe_patterns(5).iter().enumerate() {
        println!(
            "  {}. [Tick {}] [Severity {:.2}] {}",
            i + 1,
            pattern.detected_at_tick,
            pattern.severity,
            pattern.description
        );
    }

    // Display trait trends
    if let Some(last_snapshot) = metrics.snapshots.last() {
        println!("\nFinal Trait Distribution:");
        let mut traits: Vec<_> = last_snapshot.traits.iter().collect();
        traits.sort_by(|a, b| b.1.cmp(a.1));
        for (trait_item, count) in traits.iter().take(5) {
            let percentage = (**count as f32 / last_snapshot.population.total as f32) * 100.0;
            println!("  {:?}: {} agents ({:.1}%)", trait_item, count, percentage);
        }
    }

    // Export data for visualization
    println!("\nExporting data for visualization...");
    if let Err(e) = DataExporter::export_dashboard_data(&metrics, &emergence, "simulation_output") {
        eprintln!("Export error: {}", e);
    } else {
        println!("  ✓ Exported to simulation_output/");
        println!("    - metrics.json");
        println!("    - metrics.csv");
        println!("    - emergence.json");
        println!("    - emergence.csv");
        println!("    - population_trend.csv");
        println!("    - happiness_trend.csv");
        println!("    - trait_trends.csv");
        println!("    - summary.json");
    }

    println!("\n=== Example Complete ===");
    println!("\nTip: View the CSV files in a spreadsheet or plotting tool");
    println!("to visualize population dynamics and emergent patterns!");
}
