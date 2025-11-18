// src/agents/drive_satisfaction.rs
//! Drive satisfaction source tracking system
//!
//! Tracks which agents/sources satisfy which drives, enabling:
//! - Functional grief: emotional response to losing satisfaction sources
//! - Drive frustration: emotions triggered by high unsatisfied drives
//! - Social dependency: understanding who fulfills our needs

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use crate::core::DriveType;

/// Tracks satisfaction from a specific source over time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SatisfactionRecord {
    /// Who/what provides the satisfaction
    pub source_id: Uuid,
    /// Total amount satisfied over time
    pub total_satisfaction: f32,
    /// Number of times this source provided satisfaction
    pub satisfaction_count: u32,
    /// Last tick when this source provided satisfaction
    pub last_satisfaction_tick: u32,
}

impl SatisfactionRecord {
    pub fn new(source_id: Uuid) -> Self {
        Self {
            source_id,
            total_satisfaction: 0.0,
            satisfaction_count: 0,
            last_satisfaction_tick: 0,
        }
    }

    /// Record a satisfaction event
    pub fn record(&mut self, amount: f32, tick: u32) {
        self.total_satisfaction += amount;
        self.satisfaction_count += 1;
        self.last_satisfaction_tick = tick;
    }

    /// Get average satisfaction per interaction
    pub fn average_satisfaction(&self) -> f32 {
        if self.satisfaction_count == 0 {
            0.0
        } else {
            self.total_satisfaction / self.satisfaction_count as f32
        }
    }

    /// Calculate importance of this source (0.0 to 1.0)
    /// Based on frequency and amount of satisfaction
    pub fn importance(&self) -> f32 {
        // More generous scoring: even 3-5 interactions should register as important
        let frequency_score = (self.satisfaction_count as f32).min(5.0) / 5.0;
        let amount_score = (self.average_satisfaction() * 2.0).min(1.0);
        (frequency_score * 0.5 + amount_score * 0.5).min(1.0)
    }
}

/// Tracks all satisfaction sources for a specific drive
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriveSatisfactionTracker {
    pub drive_type: DriveType,
    /// Map of source_id -> satisfaction record
    pub sources: HashMap<Uuid, SatisfactionRecord>,
}

impl DriveSatisfactionTracker {
    pub fn new(drive_type: DriveType) -> Self {
        Self {
            drive_type,
            sources: HashMap::new(),
        }
    }

    /// Record satisfaction from a source
    pub fn record_satisfaction(&mut self, source_id: Uuid, amount: f32, tick: u32) {
        let record = self.sources
            .entry(source_id)
            .or_insert_with(|| SatisfactionRecord::new(source_id));
        record.record(amount, tick);
    }

    /// Get all source IDs
    pub fn get_source_ids(&self) -> Vec<Uuid> {
        self.sources.keys().copied().collect()
    }

    /// Get primary (most important) source
    pub fn get_primary_source(&self) -> Option<Uuid> {
        self.sources
            .values()
            .max_by(|a, b| {
                a.importance().partial_cmp(&b.importance()).unwrap()
            })
            .map(|record| record.source_id)
    }

    /// Get importance of a specific source
    pub fn get_source_importance(&self, source_id: Uuid) -> f32 {
        self.sources
            .get(&source_id)
            .map(|r| r.importance())
            .unwrap_or(0.0)
    }

    /// Remove a source (when they die or leave)
    pub fn remove_source(&mut self, source_id: Uuid) -> Option<SatisfactionRecord> {
        self.sources.remove(&source_id)
    }
}

/// Complete drive satisfaction tracking for an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SatisfactionTracker {
    /// Map of drive_type -> source tracker
    trackers: HashMap<DriveType, DriveSatisfactionTracker>,
}

impl SatisfactionTracker {
    pub fn new() -> Self {
        Self {
            trackers: HashMap::new(),
        }
    }

    /// Record satisfaction from a source for a drive
    pub fn record(&mut self, drive_type: DriveType, source_id: Uuid, amount: f32, tick: u32) {
        let tracker = self.trackers
            .entry(drive_type)
            .or_insert_with(|| DriveSatisfactionTracker::new(drive_type));
        tracker.record_satisfaction(source_id, amount, tick);
    }

    /// Get all sources for a drive
    pub fn get_sources(&self, drive_type: DriveType) -> Vec<Uuid> {
        self.trackers
            .get(&drive_type)
            .map(|t| t.get_source_ids())
            .unwrap_or_default()
    }

    /// Get primary source for a drive
    pub fn get_primary_source(&self, drive_type: DriveType) -> Option<Uuid> {
        self.trackers
            .get(&drive_type)
            .and_then(|t| t.get_primary_source())
    }

    /// Get importance of a source for a drive
    pub fn get_source_importance(&self, drive_type: DriveType, source_id: Uuid) -> f32 {
        self.trackers
            .get(&drive_type)
            .map(|t| t.get_source_importance(source_id))
            .unwrap_or(0.0)
    }

    /// Remove a source from all drives (when agent dies)
    pub fn remove_source(&mut self, source_id: Uuid) -> Vec<(DriveType, SatisfactionRecord)> {
        let mut removed = Vec::new();

        for (drive_type, tracker) in &mut self.trackers {
            if let Some(record) = tracker.remove_source(source_id) {
                removed.push((*drive_type, record));
            }
        }

        removed
    }

    /// Get tracker for a specific drive
    pub fn get_tracker(&self, drive_type: DriveType) -> Option<&DriveSatisfactionTracker> {
        self.trackers.get(&drive_type)
    }
}

impl Default for SatisfactionTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_satisfaction_record() {
        let source = Uuid::new_v4();
        let mut record = SatisfactionRecord::new(source);

        assert_eq!(record.satisfaction_count, 0);
        assert_eq!(record.total_satisfaction, 0.0);

        record.record(0.3, 10);
        assert_eq!(record.satisfaction_count, 1);
        assert_eq!(record.total_satisfaction, 0.3);

        record.record(0.2, 20);
        assert_eq!(record.satisfaction_count, 2);
        assert_eq!(record.total_satisfaction, 0.5);
    }

    #[test]
    fn test_average_satisfaction() {
        let source = Uuid::new_v4();
        let mut record = SatisfactionRecord::new(source);

        record.record(0.3, 10);
        record.record(0.5, 20);
        record.record(0.4, 30);

        let avg = record.average_satisfaction();
        assert!((avg - 0.4).abs() < 0.01);
    }

    #[test]
    fn test_importance_calculation() {
        let source = Uuid::new_v4();
        let mut record = SatisfactionRecord::new(source);

        // Single interaction - low importance
        record.record(0.5, 10);
        let importance1 = record.importance();

        // More interactions - higher importance
        for _ in 0..9 {
            record.record(0.5, 20);
        }
        let importance2 = record.importance();

        assert!(importance2 > importance1, "More interactions should increase importance");
    }

    #[test]
    fn test_drive_tracker() {
        let mut tracker = DriveSatisfactionTracker::new(DriveType::Social);
        let friend1 = Uuid::new_v4();
        let friend2 = Uuid::new_v4();

        tracker.record_satisfaction(friend1, 0.4, 10);
        tracker.record_satisfaction(friend2, 0.2, 10);

        let sources = tracker.get_source_ids();
        assert_eq!(sources.len(), 2);

        let primary = tracker.get_primary_source();
        assert_eq!(primary, Some(friend1), "Friend with more satisfaction should be primary");
    }

    #[test]
    fn test_satisfaction_tracker() {
        let mut tracker = SatisfactionTracker::new();
        let friend = Uuid::new_v4();
        let food_source = Uuid::new_v4();

        tracker.record(DriveType::Social, friend, 0.3, 10);
        tracker.record(DriveType::Hunger, food_source, 0.5, 10);

        let social_sources = tracker.get_sources(DriveType::Social);
        assert_eq!(social_sources.len(), 1);
        assert!(social_sources.contains(&friend));

        let hunger_sources = tracker.get_sources(DriveType::Hunger);
        assert_eq!(hunger_sources.len(), 1);
        assert!(hunger_sources.contains(&food_source));
    }

    #[test]
    fn test_remove_source() {
        let mut tracker = SatisfactionTracker::new();
        let friend = Uuid::new_v4();

        tracker.record(DriveType::Social, friend, 0.3, 10);
        tracker.record(DriveType::Reproduction, friend, 0.2, 10);

        let removed = tracker.remove_source(friend);
        assert_eq!(removed.len(), 2, "Should remove from all drives");

        let social_sources = tracker.get_sources(DriveType::Social);
        assert_eq!(social_sources.len(), 0, "Source should be removed");
    }
}
