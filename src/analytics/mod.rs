// src/analytics/mod.rs
//! Analytics, data logging, and emergence detection.

use crate::world::World;
use crate::agents::Population;

pub mod simulation_controller;
pub mod inspector;

pub use simulation_controller::{SimulationController, SimulationState};
pub use inspector::{
    Inspector, Selection, AgentInspectorData, DriveInspectorData,
    TerrainInspectorData, MemorySummary, InventorySummary, SensorySummary, SkillsSummary,
    EmotionSummary, RelationshipSummary,
};
pub mod metrics;
pub mod emergence;
pub mod export;
pub mod performance;

pub use metrics::{SimulationMetrics, TickSnapshot, PopulationSnapshot, DriveSnapshot, EmotionSnapshot};
pub use emergence::{EmergenceDetector, EmergentPattern, PatternType};
pub use export::{DataExporter, ExportFormat};
pub use performance::{PerformanceMonitor, PerformanceSnapshot};

use crate::world::World;
use crate::agents::Population;

/// Placeholder for backwards compatibility
pub struct Simulation;
pub struct SimulationConfig;
pub struct Analytics;
pub struct BehaviorAnalysis;

impl Simulation {
    pub fn new(_world: World, _population: Population) -> Self {
        Self
    }

    pub fn run_for_ticks(&mut self, _ticks: u32) {
        // Placeholder for simulation loop
    }
}
