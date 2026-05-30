// src/bevy_gui/resources/statistics.rs
//! Statistics history resource for graphs and trends.

use bevy::prelude::*;

/// A single point in the statistics history
#[derive(Debug, Clone)]
pub struct HistoryPoint {
    pub tick: u32,
    pub population: usize,
    pub average_health: f32,
    pub average_energy: f32,
    pub average_happiness: f32,
    pub total_resources: u32,
    pub buildings_completed: usize,
    pub births: u64,
    pub deaths: u64,
}

/// Statistics tab selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StatisticsTab {
    #[default]
    Population,
    Resources,
    Economy,
    Health,
}

/// Statistics history for graphing
#[derive(Resource)]
pub struct StatisticsHistory {
    pub points: Vec<HistoryPoint>,
    pub max_points: usize,
    pub sample_interval: u32,
    pub last_sample_tick: u32,
    pub active_tab: StatisticsTab,
}

impl Default for StatisticsHistory {
    fn default() -> Self {
        Self {
            points: Vec::with_capacity(500),
            max_points: 500,
            sample_interval: 10,
            last_sample_tick: 0,
            active_tab: StatisticsTab::Population,
        }
    }
}

impl StatisticsHistory {
    pub fn should_sample(&self, current_tick: u32) -> bool {
        current_tick >= self.last_sample_tick + self.sample_interval
    }

    pub fn add_point(&mut self, point: HistoryPoint) {
        self.last_sample_tick = point.tick;
        self.points.push(point);

        if self.points.len() > self.max_points {
            self.points.remove(0);
        }
    }

    pub fn clear(&mut self) {
        self.points.clear();
        self.last_sample_tick = 0;
    }

    pub fn population_data(&self) -> Vec<[f64; 2]> {
        self.points
            .iter()
            .map(|p| [p.tick as f64, p.population as f64])
            .collect()
    }

    pub fn health_data(&self) -> Vec<[f64; 2]> {
        self.points
            .iter()
            .map(|p| [p.tick as f64, p.average_health as f64])
            .collect()
    }

    pub fn energy_data(&self) -> Vec<[f64; 2]> {
        self.points
            .iter()
            .map(|p| [p.tick as f64, p.average_energy as f64])
            .collect()
    }

    pub fn happiness_data(&self) -> Vec<[f64; 2]> {
        self.points
            .iter()
            .map(|p| [p.tick as f64, p.average_happiness as f64])
            .collect()
    }

    pub fn resources_data(&self) -> Vec<[f64; 2]> {
        self.points
            .iter()
            .map(|p| [p.tick as f64, p.total_resources as f64])
            .collect()
    }
}

/// Alias for compatibility
pub type StatisticsData = StatisticsHistory;
