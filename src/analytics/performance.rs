// src/analytics/performance.rs
//! Performance monitoring and profiling for simulation optimization.

use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use std::collections::BTreeMap;

/// Performance metrics for a single operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationMetrics {
    pub total_calls: u64,
    pub total_duration_micros: u64,
    pub average_duration_micros: f64,
    pub min_duration_micros: u64,
    pub max_duration_micros: u64,
}

impl OperationMetrics {
    fn new() -> Self {
        Self {
            total_calls: 0,
            total_duration_micros: 0,
            average_duration_micros: 0.0,
            min_duration_micros: u64::MAX,
            max_duration_micros: 0,
        }
    }

    fn record(&mut self, duration: Duration) {
        let micros = duration.as_micros() as u64;

        self.total_calls += 1;
        self.total_duration_micros += micros;
        self.average_duration_micros = self.total_duration_micros as f64 / self.total_calls as f64;
        self.min_duration_micros = self.min_duration_micros.min(micros);
        self.max_duration_micros = self.max_duration_micros.max(micros);
    }
}

/// Snapshot of performance at a specific turn
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceSnapshot {
    pub turn: u32,
    pub turn_duration_micros: u64,
    pub turns_per_second: f64,
    pub agents_per_turn: usize,
    pub memory_usage_estimate_kb: usize,
}

/// Performance monitor for tracking simulation performance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMonitor {
    pub operations: BTreeMap<String, OperationMetrics>,
    pub snapshots: Vec<PerformanceSnapshot>,
    pub max_snapshots: usize,

    #[serde(skip)]
    pub last_turn_start: Option<Instant>,
}

impl PerformanceMonitor {
    pub fn new(max_snapshots: usize) -> Self {
        Self {
            operations: BTreeMap::new(),
            snapshots: Vec::new(),
            max_snapshots,
            last_turn_start: None,
        }
    }

    /// Start timing an operation
    pub fn start_operation(&mut self, _name: &str) -> Instant {
        Instant::now()
    }

    /// End timing an operation
    pub fn end_operation(&mut self, name: &str, start: Instant) {
        let duration = start.elapsed();
        self.operations
            .entry(name.to_string())
            .or_insert_with(OperationMetrics::new)
            .record(duration);
    }

    /// Start timing a turn
    pub fn start_turn(&mut self) {
        self.last_turn_start = Some(Instant::now());
    }

    /// End timing a turn and record snapshot
    pub fn end_turn(&mut self, turn: u32, agent_count: usize) {
        if let Some(start) = self.last_turn_start {
            let duration = start.elapsed();
            let micros = duration.as_micros() as u64;

            let turns_per_second = if micros > 0 {
                1_000_000.0 / micros as f64
            } else {
                0.0
            };

            // Rough memory estimate (not accurate, just for trending)
            let memory_estimate = agent_count * 10; // ~10KB per agent estimate

            let snapshot = PerformanceSnapshot {
                turn,
                turn_duration_micros: micros,
                turns_per_second,
                agents_per_turn: agent_count,
                memory_usage_estimate_kb: memory_estimate,
            };

            self.snapshots.push(snapshot);

            // Keep only recent snapshots
            if self.snapshots.len() > self.max_snapshots {
                self.snapshots.remove(0);
            }
        }

        self.last_turn_start = None;
    }

    /// Get average turns per second over all snapshots
    pub fn average_turns_per_second(&self) -> f64 {
        if self.snapshots.is_empty() {
            return 0.0;
        }

        let sum: f64 = self.snapshots.iter().map(|s| s.turns_per_second).sum();
        sum / self.snapshots.len() as f64
    }

    /// Get slowest operations
    pub fn slowest_operations(&self, count: usize) -> Vec<(&str, &OperationMetrics)> {
        let mut ops: Vec<_> = self
            .operations
            .iter()
            .map(|(name, metrics)| (name.as_str(), metrics))
            .collect();

        ops.sort_by(|a, b| {
            b.1.average_duration_micros
                .partial_cmp(&a.1.average_duration_micros)
                .unwrap()
        });

        ops.into_iter().take(count).collect()
    }


    /// Get performance summary
    pub fn summary(&self) -> PerformanceSummary {
        let total_operations: u64 = self.operations.values().map(|m| m.total_calls).sum();

        let total_time_micros: u64 = self
            .operations
            .values()
            .map(|m| m.total_duration_micros)
            .sum();

        let avg_tps = self.average_turns_per_second();

        let peak_tps = self
            .snapshots
            .iter()
            .map(|s| s.turns_per_second)
            .fold(0.0_f64, f64::max);

        let min_tps = self
            .snapshots
            .iter()
            .map(|s| s.turns_per_second)
            .fold(f64::INFINITY, f64::min);

        PerformanceSummary {
            total_operations,
            total_time_seconds: total_time_micros as f64 / 1_000_000.0,
            average_turns_per_second: avg_tps,
            peak_turns_per_second: peak_tps,
            min_turns_per_second: if min_tps == f64::INFINITY { 0.0 } else { min_tps },
            total_turns_measured: self.snapshots.len(),
        }
    }

    /// Reset all metrics
    pub fn reset(&mut self) {
        self.operations.clear();
        self.snapshots.clear();
        self.last_turn_start = None;
    }
}

impl Default for PerformanceMonitor {
    fn default() -> Self {
        Self::new(1000)
    }
}

/// Summary of performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceSummary {
    pub total_operations: u64,
    pub total_time_seconds: f64,
    pub average_turns_per_second: f64,
    pub peak_turns_per_second: f64,
    pub min_turns_per_second: f64,
    pub total_turns_measured: usize,
}

/// Macro for timing operations
#[macro_export]
macro_rules! profile_operation {
    ($monitor:expr, $name:expr, $block:block) => {{
        let start = $monitor.start_operation($name);
        let result = $block;
        $monitor.end_operation($name, start);
        result
    }};
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_performance_monitor_creation() {
        let monitor = PerformanceMonitor::new(100);
        assert_eq!(monitor.max_snapshots, 100);
        assert_eq!(monitor.operations.len(), 0);
        assert_eq!(monitor.snapshots.len(), 0);
    }

    #[test]
    fn test_operation_timing() {
        let mut monitor = PerformanceMonitor::new(100);

        let start = monitor.start_operation("test_op");
        thread::sleep(Duration::from_millis(1));
        monitor.end_operation("test_op", start);

        assert_eq!(monitor.operations.len(), 1);
        let metrics = monitor.operations.get("test_op").unwrap();
        assert_eq!(metrics.total_calls, 1);
        assert!(metrics.average_duration_micros > 0.0);
    }

    #[test]
    fn test_turn_timing() {
        let mut monitor = PerformanceMonitor::new(100);

        monitor.start_turn();
        thread::sleep(Duration::from_millis(1));
        monitor.end_turn(0, 10);

        assert_eq!(monitor.snapshots.len(), 1);
        assert_eq!(monitor.snapshots[0].turn, 0);
        assert_eq!(monitor.snapshots[0].agents_per_turn, 10);
        assert!(monitor.snapshots[0].turn_duration_micros > 0);
    }

    #[test]
    fn test_max_snapshots_limit() {
        let mut monitor = PerformanceMonitor::new(5);

        for i in 0..10 {
            monitor.start_turn();
            monitor.end_turn(i, 1);
        }

        assert_eq!(monitor.snapshots.len(), 5); // Should keep only last 5
        assert_eq!(monitor.snapshots.first().unwrap().turn, 5);
        assert_eq!(monitor.snapshots.last().unwrap().turn, 9);
    }

    #[test]
    fn test_average_turns_per_second() {
        let mut monitor = PerformanceMonitor::new(100);

        monitor.start_turn();
        thread::sleep(Duration::from_millis(10));
        monitor.end_turn(0, 1);

        monitor.start_turn();
        thread::sleep(Duration::from_millis(10));
        monitor.end_turn(1, 1);

        let avg_tps = monitor.average_turns_per_second();
        assert!(avg_tps > 0.0);
        assert!(avg_tps < 200.0); // Should be roughly 100 TPS with 10ms per turn
    }

    #[test]
    fn test_slowest_operations() {
        let mut monitor = PerformanceMonitor::new(100);

        // Fast operation
        let start = monitor.start_operation("fast");
        monitor.end_operation("fast", start);

        // Slow operation
        let start = monitor.start_operation("slow");
        thread::sleep(Duration::from_millis(2));
        monitor.end_operation("slow", start);

        let slowest = monitor.slowest_operations(1);
        assert_eq!(slowest.len(), 1);
        assert_eq!(slowest[0].0, "slow");
    }

    #[test]
    fn test_summary() {
        let mut monitor = PerformanceMonitor::new(100);

        monitor.start_turn();
        thread::sleep(Duration::from_millis(1));
        monitor.end_turn(0, 10);

        let summary = monitor.summary();
        assert!(summary.average_turns_per_second > 0.0);
        assert_eq!(summary.total_turns_measured, 1);
    }

    #[test]
    fn test_reset() {
        let mut monitor = PerformanceMonitor::new(100);

        let start = monitor.start_operation("test");
        monitor.end_operation("test", start);

        monitor.start_turn();
        monitor.end_turn(0, 1);

        assert_eq!(monitor.operations.len(), 1);
        assert_eq!(monitor.snapshots.len(), 1);

        monitor.reset();

        assert_eq!(monitor.operations.len(), 0);
        assert_eq!(monitor.snapshots.len(), 0);
    }
}
