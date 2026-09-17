// src/bevy_gui/resources/statistics.rs
//! Statistics history resource for graphs and trends.

use bevy::prelude::*;

/// A single point in the statistics history
#[derive(Debug, Clone)]
pub struct HistoryPoint {
    pub turn: u32,
    pub population: usize,
    /// Population by life stage, as recorded at this turn
    pub infants: usize,
    pub children: usize,
    pub adolescents: usize,
    pub adults: usize,
    pub elderly: usize,
    pub average_health: f32,
    pub average_energy: f32,
    pub average_happiness: f32,
    pub total_resources: u32,
    pub buildings_completed: usize,
    /// Buildings still under construction at this turn
    pub buildings_construction: usize,
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
    pub last_sample_turn: u32,
    pub active_tab: StatisticsTab,
}

impl Default for StatisticsHistory {
    fn default() -> Self {
        Self {
            points: Vec::with_capacity(500),
            max_points: 500,
            sample_interval: 10,
            last_sample_turn: 0,
            active_tab: StatisticsTab::Population,
        }
    }
}

impl StatisticsHistory {
    pub fn should_sample(&self, current_turn: u32) -> bool {
        current_turn >= self.last_sample_turn + self.sample_interval
    }

    pub fn add_point(&mut self, point: HistoryPoint) {
        self.last_sample_turn = point.turn;
        self.points.push(point);

        if self.points.len() > self.max_points {
            self.points.remove(0);
        }
    }

    pub fn clear(&mut self) {
        self.points.clear();
        self.last_sample_turn = 0;
    }

    pub fn population_data(&self) -> Vec<[f64; 2]> {
        self.points
            .iter()
            .map(|p| [p.turn as f64, p.population as f64])
            .collect()
    }

    pub fn health_data(&self) -> Vec<[f64; 2]> {
        self.points
            .iter()
            .map(|p| [p.turn as f64, p.average_health as f64])
            .collect()
    }

    pub fn energy_data(&self) -> Vec<[f64; 2]> {
        self.points
            .iter()
            .map(|p| [p.turn as f64, p.average_energy as f64])
            .collect()
    }

    pub fn happiness_data(&self) -> Vec<[f64; 2]> {
        self.points
            .iter()
            .map(|p| [p.turn as f64, p.average_happiness as f64])
            .collect()
    }

    pub fn resources_data(&self) -> Vec<[f64; 2]> {
        self.points
            .iter()
            .map(|p| [p.turn as f64, p.total_resources as f64])
            .collect()
    }
}

/// Alias for compatibility
pub type StatisticsData = StatisticsHistory;
