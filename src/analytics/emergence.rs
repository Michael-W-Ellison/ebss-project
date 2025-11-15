// src/analytics/emergence.rs
//! Algorithms for detecting emergent patterns in simulation behavior.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::agents::Population;
use crate::analytics::metrics::SimulationMetrics;
use crate::core::{Trait, DriveType, EmotionType};

/// Type of emergent pattern detected
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PatternType {
    /// Trait clustering (specific traits becoming dominant)
    TraitClustering { trait_item: Trait, prevalence: u32 },
    /// Population boom (rapid growth)
    PopulationBoom { growth_rate: f32 },
    /// Population collapse (rapid decline)
    PopulationCollapse { decline_rate: f32 },
    /// Social polarization (high conflict)
    SocialPolarization { conflict_rate: f32 },
    /// Harmonic society (high cooperation)
    HarmonicSociety { cooperation_rate: f32 },
    /// Drive crisis (many agents with critical drive)
    DriveCrisis { drive: DriveType, critical_percentage: f32 },
    /// Emotional epidemic (emotion spreading through population)
    EmotionalEpidemic { emotion: EmotionType, intensity: f32 },
    /// Mass migration (high abandonment rate)
    MassMigration { abandonment_rate: f32 },
    /// Stable equilibrium (minimal change over time)
    StableEquilibrium { stability_score: f32 },
}

/// Detected emergent pattern with context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmergentPattern {
    pub pattern_type: PatternType,
    pub detected_at_tick: u32,
    pub severity: f32,      // 0.0 to 1.0
    pub description: String,
}

/// Emergence detection engine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmergenceDetector {
    pub detected_patterns: Vec<EmergentPattern>,
    pub detection_threshold: f32, // Minimum severity to report (default 0.5)
}

impl EmergenceDetector {
    pub fn new() -> Self {
        Self {
            detected_patterns: Vec::new(),
            detection_threshold: 0.5,
        }
    }

    /// Analyze metrics and detect emergent patterns
    pub fn detect_patterns(&mut self, metrics: &SimulationMetrics, current_tick: u32) {
        if metrics.snapshots.len() < 2 {
            return; // Need at least 2 snapshots for trend analysis
        }

        self.detect_trait_clustering(metrics, current_tick);
        self.detect_population_changes(metrics, current_tick);
        self.detect_social_patterns(metrics, current_tick);
        self.detect_drive_crises(metrics, current_tick);
        self.detect_emotional_epidemics(metrics, current_tick);
        self.detect_stability(metrics, current_tick);
    }

    fn detect_trait_clustering(&mut self, metrics: &SimulationMetrics, current_tick: u32) {
        if let Some(last_snapshot) = metrics.snapshots.last() {
            let total_pop = last_snapshot.population.total;
            if total_pop == 0 {
                return;
            }

            for (trait_item, &count) in &last_snapshot.traits {
                let prevalence = (count as f32 / total_pop as f32) * 100.0;

                // Detect if a trait is present in >60% of population
                if prevalence > 60.0 {
                    let severity = ((prevalence - 60.0) / 40.0).min(1.0);

                    if severity >= self.detection_threshold {
                        self.report_pattern(EmergentPattern {
                            pattern_type: PatternType::TraitClustering {
                                trait_item: trait_item.clone(),
                                prevalence: count,
                            },
                            detected_at_tick: current_tick,
                            severity,
                            description: format!(
                                "Trait {:?} has clustered to {:.1}% of population ({} agents)",
                                trait_item, prevalence, count
                            ),
                        });
                    }
                }
            }
        }
    }

    fn detect_population_changes(&mut self, metrics: &SimulationMetrics, current_tick: u32) {
        let len = metrics.snapshots.len();
        if len < 5 {
            return; // Need more history
        }

        // Look at last 5 snapshots
        let recent = &metrics.snapshots[len - 5..];
        let first_pop = recent.first().unwrap().population.total as f32;
        let last_pop = recent.last().unwrap().population.total as f32;

        if first_pop == 0.0 {
            return;
        }

        let change_rate = (last_pop - first_pop) / first_pop;

        // Boom: >50% growth in 5 intervals
        if change_rate > 0.5 {
            let severity = (change_rate / 2.0).min(1.0); // Cap at 1.0 for 200% growth
            if severity >= self.detection_threshold {
                self.report_pattern(EmergentPattern {
                    pattern_type: PatternType::PopulationBoom {
                        growth_rate: change_rate,
                    },
                    detected_at_tick: current_tick,
                    severity,
                    description: format!(
                        "Population boom: {:.1}% growth (from {} to {} agents)",
                        change_rate * 100.0,
                        first_pop,
                        last_pop
                    ),
                });
            }
        }

        // Collapse: >30% decline in 5 intervals
        if change_rate < -0.3 {
            let severity = (change_rate.abs() / 0.7).min(1.0); // Cap at 1.0 for 100% loss
            if severity >= self.detection_threshold {
                self.report_pattern(EmergentPattern {
                    pattern_type: PatternType::PopulationCollapse {
                        decline_rate: change_rate,
                    },
                    detected_at_tick: current_tick,
                    severity,
                    description: format!(
                        "Population collapse: {:.1}% decline (from {} to {} agents)",
                        change_rate * 100.0,
                        first_pop,
                        last_pop
                    ),
                });
            }
        }
    }

    fn detect_social_patterns(&mut self, metrics: &SimulationMetrics, current_tick: u32) {
        if let Some(last_snapshot) = metrics.snapshots.last() {
            let relationships = &last_snapshot.relationships;
            if relationships.total_relationships == 0 {
                return;
            }

            let conflict_rate = relationships.conflicts as f32 / relationships.total_relationships as f32;

            // Polarization: >40% conflicts
            if conflict_rate > 0.4 {
                let severity = ((conflict_rate - 0.4) / 0.6).min(1.0);
                if severity >= self.detection_threshold {
                    self.report_pattern(EmergentPattern {
                        pattern_type: PatternType::SocialPolarization { conflict_rate },
                        detected_at_tick: current_tick,
                        severity,
                        description: format!(
                            "Social polarization: {:.1}% of relationships are negative",
                            conflict_rate * 100.0
                        ),
                    });
                }
            }

            // Harmony: >80% positive relationships AND high average trust
            let positive_rate = 1.0 - conflict_rate;
            if positive_rate > 0.8 && relationships.average_trust > 0.6 {
                let severity = ((positive_rate - 0.8) / 0.2 + relationships.average_trust) / 2.0;
                if severity >= self.detection_threshold {
                    self.report_pattern(EmergentPattern {
                        pattern_type: PatternType::HarmonicSociety {
                            cooperation_rate: positive_rate,
                        },
                        detected_at_tick: current_tick,
                        severity,
                        description: format!(
                            "Harmonic society: {:.1}% positive relationships with avg trust {:.2}",
                            positive_rate * 100.0,
                            relationships.average_trust
                        ),
                    });
                }
            }
        }
    }

    fn detect_drive_crises(&mut self, metrics: &SimulationMetrics, current_tick: u32) {
        if let Some(last_snapshot) = metrics.snapshots.last() {
            let total_pop = last_snapshot.population.total;
            if total_pop == 0 {
                return;
            }

            for (drive_type, drive_snapshot) in &last_snapshot.drives {
                let critical_percentage = drive_snapshot.critical_count as f32 / total_pop as f32;

                // Crisis: >30% of agents have critical drive level
                if critical_percentage > 0.3 {
                    let severity = ((critical_percentage - 0.3) / 0.7).min(1.0);
                    if severity >= self.detection_threshold {
                        self.report_pattern(EmergentPattern {
                            pattern_type: PatternType::DriveCrisis {
                                drive: drive_type.clone(),
                                critical_percentage,
                            },
                            detected_at_tick: current_tick,
                            severity,
                            description: format!(
                                "{:?} crisis: {:.1}% of agents have critically low {:?} (avg: {:.2})",
                                drive_type,
                                critical_percentage * 100.0,
                                drive_type,
                                drive_snapshot.average_value
                            ),
                        });
                    }
                }
            }
        }
    }

    fn detect_emotional_epidemics(&mut self, metrics: &SimulationMetrics, current_tick: u32) {
        if let Some(last_snapshot) = metrics.snapshots.last() {
            for (emotion_type, emotion_snapshot) in &last_snapshot.emotions {
                let intensity = emotion_snapshot.average_value.abs();

                // Epidemic: Strong emotion across population (avg > 0.6 or < -0.6)
                if intensity > 0.6 {
                    let severity = ((intensity - 0.6) / 0.4).min(1.0);
                    if severity >= self.detection_threshold {
                        let emotion_direction = if emotion_snapshot.average_value > 0.0 {
                            "positive"
                        } else {
                            "negative"
                        };

                        self.report_pattern(EmergentPattern {
                            pattern_type: PatternType::EmotionalEpidemic {
                                emotion: emotion_type.clone(),
                                intensity,
                            },
                            detected_at_tick: current_tick,
                            severity,
                            description: format!(
                                "{:?} epidemic: {} {:?} spreading through population (avg: {:.2})",
                                emotion_type, emotion_direction, emotion_type, emotion_snapshot.average_value
                            ),
                        });
                    }
                }
            }
        }
    }

    fn detect_stability(&mut self, metrics: &SimulationMetrics, current_tick: u32) {
        let len = metrics.snapshots.len();
        if len < 10 {
            return; // Need more history
        }

        // Look at last 10 snapshots
        let recent = &metrics.snapshots[len - 10..];

        // Calculate variance in population size
        let populations: Vec<f32> = recent.iter().map(|s| s.population.total as f32).collect();
        let avg_pop = populations.iter().sum::<f32>() / populations.len() as f32;

        if avg_pop == 0.0 {
            return;
        }

        let variance: f32 = populations
            .iter()
            .map(|&p| {
                let diff = p - avg_pop;
                diff * diff
            })
            .sum::<f32>()
            / populations.len() as f32;

        let coefficient_of_variation = (variance.sqrt() / avg_pop).abs();

        // Stable: CV < 0.05 (less than 5% variation)
        if coefficient_of_variation < 0.05 {
            let stability_score = 1.0 - (coefficient_of_variation / 0.05);

            if stability_score >= self.detection_threshold {
                self.report_pattern(EmergentPattern {
                    pattern_type: PatternType::StableEquilibrium { stability_score },
                    detected_at_tick: current_tick,
                    severity: stability_score,
                    description: format!(
                        "Stable equilibrium: Population variance only {:.2}% over last {} snapshots",
                        coefficient_of_variation * 100.0,
                        recent.len()
                    ),
                });
            }
        }
    }

    fn report_pattern(&mut self, pattern: EmergentPattern) {
        // Avoid duplicate reports for same pattern type within short time
        let is_duplicate = self.detected_patterns.iter().rev().take(5).any(|p| {
            std::mem::discriminant(&p.pattern_type) == std::mem::discriminant(&pattern.pattern_type)
                && pattern.detected_at_tick - p.detected_at_tick < 100
        });

        if !is_duplicate {
            self.detected_patterns.push(pattern);
        }
    }

    /// Get patterns detected in a specific time range
    pub fn patterns_in_range(&self, start_tick: u32, end_tick: u32) -> Vec<&EmergentPattern> {
        self.detected_patterns
            .iter()
            .filter(|p| p.detected_at_tick >= start_tick && p.detected_at_tick <= end_tick)
            .collect()
    }

    /// Get patterns of a specific type
    pub fn patterns_by_type(&self, pattern_discriminant: std::mem::Discriminant<PatternType>) -> Vec<&EmergentPattern> {
        self.detected_patterns
            .iter()
            .filter(|p| std::mem::discriminant(&p.pattern_type) == pattern_discriminant)
            .collect()
    }

    /// Get most severe patterns
    pub fn most_severe_patterns(&self, count: usize) -> Vec<&EmergentPattern> {
        let mut patterns: Vec<&EmergentPattern> = self.detected_patterns.iter().collect();
        patterns.sort_by(|a, b| b.severity.partial_cmp(&a.severity).unwrap());
        patterns.into_iter().take(count).collect()
    }
}

impl Default for EmergenceDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::{Population, AgentConfig};
    use crate::analytics::metrics::SimulationMetrics;

    #[test]
    fn test_detector_creation() {
        let detector = EmergenceDetector::new();
        assert_eq!(detector.detected_patterns.len(), 0);
        assert_eq!(detector.detection_threshold, 0.5);
    }

    #[test]
    fn test_detect_with_insufficient_data() {
        let mut detector = EmergenceDetector::new();
        let metrics = SimulationMetrics::new(1, 100);

        // Should not crash with empty metrics
        detector.detect_patterns(&metrics, 0);
        assert_eq!(detector.detected_patterns.len(), 0);
    }

    #[test]
    fn test_patterns_in_range() {
        let mut detector = EmergenceDetector::new();

        detector.detected_patterns.push(EmergentPattern {
            pattern_type: PatternType::StableEquilibrium {
                stability_score: 0.9,
            },
            detected_at_tick: 50,
            severity: 0.9,
            description: "Test".to_string(),
        });

        detector.detected_patterns.push(EmergentPattern {
            pattern_type: PatternType::StableEquilibrium {
                stability_score: 0.8,
            },
            detected_at_tick: 150,
            severity: 0.8,
            description: "Test".to_string(),
        });

        let patterns = detector.patterns_in_range(40, 100);
        assert_eq!(patterns.len(), 1);
        assert_eq!(patterns[0].detected_at_tick, 50);
    }

    #[test]
    fn test_most_severe_patterns() {
        let mut detector = EmergenceDetector::new();

        detector.detected_patterns.push(EmergentPattern {
            pattern_type: PatternType::StableEquilibrium {
                stability_score: 0.5,
            },
            detected_at_tick: 0,
            severity: 0.5,
            description: "Low".to_string(),
        });

        detector.detected_patterns.push(EmergentPattern {
            pattern_type: PatternType::StableEquilibrium {
                stability_score: 0.9,
            },
            detected_at_tick: 0,
            severity: 0.9,
            description: "High".to_string(),
        });

        let severe = detector.most_severe_patterns(1);
        assert_eq!(severe.len(), 1);
        assert_eq!(severe[0].severity, 0.9);
    }
}
