// src/analytics/emergence.rs
//! Algorithms for detecting emergent patterns in simulation behavior.
//!
//! This module provides sophisticated emergence detection capabilities including:
//! - 15+ pattern types across social, demographic, and behavioral dimensions
//! - Calibration system for adjusting detection thresholds based on training data
//! - Pattern prediction based on trend analysis
//! - Curiosity and exploration emergence patterns
//! - Compound pattern detection (multiple related patterns)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
    /// Discovery boom (rapid increase in discoveries)
    DiscoveryBoom { discovery_rate: f32, discovery_type: Option<String> },
    /// Exploration surge (high exploration activity)
    ExplorationSurge { exploration_rate: f32 },
    /// Exploration decline (exploration activity dropping)
    ExplorationDecline { decline_rate: f32 },
    /// Curiosity awakening (many agents developing high curiosity)
    CuriosityAwakening { high_curiosity_rate: f32 },
    /// Knowledge saturation (exploration efficiency declining)
    KnowledgeSaturation { efficiency_drop: f32 },
    /// Compound crisis (multiple crises occurring together)
    CompoundCrisis { crisis_types: Vec<DriveType>, severity: f32 },
}

/// Calibration thresholds for emergence detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionThresholds {
    /// Trait clustering threshold (% of population)
    pub trait_clustering: f32,
    /// Population boom threshold (% growth rate)
    pub population_boom: f32,
    /// Population collapse threshold (% decline rate)
    pub population_collapse: f32,
    /// Social polarization threshold (% conflict rate)
    pub social_polarization: f32,
    /// Harmonic society threshold (% positive rate)
    pub harmonic_society: f32,
    /// Drive crisis threshold (% critical agents)
    pub drive_crisis: f32,
    /// Emotional epidemic threshold (intensity)
    pub emotional_epidemic: f32,
    /// Mass migration threshold (% abandonment rate)
    pub mass_migration: f32,
    /// Stability coefficient of variation threshold
    pub stability: f32,
    /// Discovery boom threshold (rate multiplier)
    pub discovery_boom: f32,
    /// Exploration surge threshold (rate)
    pub exploration_surge: f32,
    /// Curiosity awakening threshold (% high curiosity)
    pub curiosity_awakening: f32,
}

impl Default for DetectionThresholds {
    fn default() -> Self {
        Self {
            trait_clustering: 0.60,
            population_boom: 0.50,
            population_collapse: 0.30,
            social_polarization: 0.40,
            harmonic_society: 0.80,
            drive_crisis: 0.30,
            emotional_epidemic: 0.60,
            mass_migration: 0.15,
            stability: 0.05,
            discovery_boom: 2.0,
            exploration_surge: 0.50,
            curiosity_awakening: 0.40,
        }
    }
}

/// Training sample for calibrating thresholds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingSample {
    /// Pattern type that was observed
    pub pattern_type: String,
    /// Metric values when pattern occurred
    pub metric_values: HashMap<String, f32>,
    /// Was this a true positive detection?
    pub is_positive: bool,
    /// Severity rating from training data
    pub rated_severity: f32,
}

/// Result of threshold calibration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalibrationResult {
    pub success: bool,
    pub adjustments: HashMap<String, f32>,
    pub message: String,
}

/// Direction of a trend
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrendDirection {
    Increasing,
    Decreasing,
    Stable,
}

/// Predicted future pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternPrediction {
    pub predicted_pattern: String,
    pub confidence: f32,
    pub estimated_ticks_until: u32,
    pub trend_direction: TrendDirection,
}

/// Detected emergent pattern with context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmergentPattern {
    pub pattern_type: PatternType,
    pub detected_at_tick: u32,
    pub severity: f32,      // 0.0 to 1.0
    pub description: String,
}

/// Emergence detection engine with calibration support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmergenceDetector {
    pub detected_patterns: Vec<EmergentPattern>,
    pub detection_threshold: f32, // Minimum severity to report (default 0.5)
    pub thresholds: DetectionThresholds,
    training_samples: Vec<TrainingSample>,
    /// Track discovery counts for boom detection
    previous_discoveries: Option<usize>,
    /// Track exploration counts for surge detection
    previous_explorations: Option<u32>,
    /// Track previous efficiency for saturation detection
    previous_efficiency: Option<f32>,
}

impl EmergenceDetector {
    pub fn new() -> Self {
        Self {
            detected_patterns: Vec::new(),
            detection_threshold: 0.5,
            thresholds: DetectionThresholds::default(),
            training_samples: Vec::new(),
            previous_discoveries: None,
            previous_explorations: None,
            previous_efficiency: None,
        }
    }

    /// Create detector with custom thresholds
    pub fn with_thresholds(thresholds: DetectionThresholds) -> Self {
        Self {
            thresholds,
            ..Self::new()
        }
    }

    /// Analyze metrics and detect emergent patterns
    pub fn detect_patterns(&mut self, metrics: &SimulationMetrics, current_tick: u32) {
        if metrics.snapshots.len() < 2 {
            return; // Need at least 2 snapshots for trend analysis
        }

        // Original pattern detection
        self.detect_trait_clustering(metrics, current_tick);
        self.detect_population_changes(metrics, current_tick);
        self.detect_social_patterns(metrics, current_tick);
        self.detect_drive_crises(metrics, current_tick);
        self.detect_emotional_epidemics(metrics, current_tick);
        self.detect_stability(metrics, current_tick);

        // New pattern detection
        self.detect_mass_migration(metrics, current_tick);
        self.detect_curiosity_patterns(metrics, current_tick);
        self.detect_compound_crises(metrics, current_tick);
    }

    /// Add a training sample for threshold calibration
    pub fn add_training_sample(&mut self, sample: TrainingSample) {
        self.training_samples.push(sample);
    }

    /// Calibrate thresholds based on training data
    pub fn calibrate_from_training(&mut self) -> CalibrationResult {
        if self.training_samples.is_empty() {
            return CalibrationResult {
                success: false,
                adjustments: HashMap::new(),
                message: "No training samples available".to_string(),
            };
        }

        let mut adjustments = HashMap::new();

        // Group samples by pattern type
        let mut samples_by_type: HashMap<String, Vec<&TrainingSample>> = HashMap::new();
        for sample in &self.training_samples {
            samples_by_type
                .entry(sample.pattern_type.clone())
                .or_default()
                .push(sample);
        }

        // Calibrate each pattern type
        for (pattern_type, samples) in samples_by_type {
            let positive_samples: Vec<_> = samples.iter().filter(|s| s.is_positive).collect();
            let negative_samples: Vec<_> = samples.iter().filter(|s| !s.is_positive).collect();

            if positive_samples.is_empty() || negative_samples.is_empty() {
                continue;
            }

            // Find optimal threshold as midpoint between positive and negative means
            let key = format!("{}_threshold", pattern_type.to_lowercase());
            if let Some(values_key) = samples[0].metric_values.keys().next() {
                let positive_mean: f32 = positive_samples.iter()
                    .filter_map(|s| s.metric_values.get(values_key))
                    .sum::<f32>() / positive_samples.len() as f32;

                let negative_mean: f32 = negative_samples.iter()
                    .filter_map(|s| s.metric_values.get(values_key))
                    .sum::<f32>() / negative_samples.len() as f32;

                let optimal_threshold = (positive_mean + negative_mean) / 2.0;
                adjustments.insert(key, optimal_threshold);
            }
        }

        // Apply adjustments to thresholds
        self.apply_calibration(&adjustments);

        CalibrationResult {
            success: true,
            adjustments,
            message: format!("Calibrated from {} training samples", self.training_samples.len()),
        }
    }

    fn apply_calibration(&mut self, adjustments: &HashMap<String, f32>) {
        if let Some(&v) = adjustments.get("trait_clustering_threshold") {
            self.thresholds.trait_clustering = v;
        }
        if let Some(&v) = adjustments.get("population_boom_threshold") {
            self.thresholds.population_boom = v;
        }
        if let Some(&v) = adjustments.get("population_collapse_threshold") {
            self.thresholds.population_collapse = v;
        }
        if let Some(&v) = adjustments.get("social_polarization_threshold") {
            self.thresholds.social_polarization = v;
        }
        if let Some(&v) = adjustments.get("drive_crisis_threshold") {
            self.thresholds.drive_crisis = v;
        }
        if let Some(&v) = adjustments.get("mass_migration_threshold") {
            self.thresholds.mass_migration = v;
        }
        if let Some(&v) = adjustments.get("discovery_boom_threshold") {
            self.thresholds.discovery_boom = v;
        }
    }

    /// Predict potential patterns based on current trends
    pub fn predict_patterns(&self, metrics: &SimulationMetrics) -> Vec<PatternPrediction> {
        let mut predictions = Vec::new();

        if metrics.snapshots.len() < 5 {
            return predictions;
        }

        // Predict population changes
        if let Some(prediction) = self.predict_population_trend(metrics) {
            predictions.push(prediction);
        }

        // Predict drive crises
        if let Some(prediction) = self.predict_drive_crisis(metrics) {
            predictions.push(prediction);
        }

        // Predict exploration decline
        if let Some(prediction) = self.predict_exploration_decline(metrics) {
            predictions.push(prediction);
        }

        predictions
    }

    fn predict_population_trend(&self, metrics: &SimulationMetrics) -> Option<PatternPrediction> {
        let len = metrics.snapshots.len();
        if len < 5 {
            return None;
        }

        let recent = &metrics.snapshots[len - 5..];
        let pops: Vec<f32> = recent.iter().map(|s| s.population.total as f32).collect();

        // Calculate trend using linear regression
        let n = pops.len() as f32;
        let x_sum: f32 = (0..pops.len()).map(|i| i as f32).sum();
        let y_sum: f32 = pops.iter().sum();
        let xy_sum: f32 = pops.iter().enumerate().map(|(i, y)| i as f32 * y).sum();
        let xx_sum: f32 = (0..pops.len()).map(|i| (i * i) as f32).sum();

        let slope = (n * xy_sum - x_sum * y_sum) / (n * xx_sum - x_sum * x_sum);
        let avg_pop = y_sum / n;

        if avg_pop == 0.0 {
            return None;
        }

        let relative_slope = slope / avg_pop;

        if relative_slope > 0.1 {
            Some(PatternPrediction {
                predicted_pattern: "PopulationBoom".to_string(),
                confidence: (relative_slope * 2.0).min(1.0),
                estimated_ticks_until: 50,
                trend_direction: TrendDirection::Increasing,
            })
        } else if relative_slope < -0.1 {
            Some(PatternPrediction {
                predicted_pattern: "PopulationCollapse".to_string(),
                confidence: (relative_slope.abs() * 2.0).min(1.0),
                estimated_ticks_until: 50,
                trend_direction: TrendDirection::Decreasing,
            })
        } else {
            None
        }
    }

    fn predict_drive_crisis(&self, metrics: &SimulationMetrics) -> Option<PatternPrediction> {
        let len = metrics.snapshots.len();
        if len < 3 {
            return None;
        }

        let recent = &metrics.snapshots[len - 3..];

        // Find drive with increasing critical count
        for (drive_type, _) in recent.last()?.drives.iter() {
            let critical_rates: Vec<f32> = recent.iter().map(|s| {
                let total = s.population.total as f32;
                if total == 0.0 { return 0.0; }
                s.drives.get(drive_type).map(|d| d.critical_count as f32 / total).unwrap_or(0.0)
            }).collect();

            // Check if trending upward
            if critical_rates.len() >= 2 {
                let trend = critical_rates.last().unwrap() - critical_rates.first().unwrap();
                let current = *critical_rates.last().unwrap();

                if trend > 0.05 && current > 0.15 && current < self.thresholds.drive_crisis {
                    return Some(PatternPrediction {
                        predicted_pattern: format!("{:?}Crisis", drive_type),
                        confidence: ((current / self.thresholds.drive_crisis) * 0.8).min(0.9),
                        estimated_ticks_until: ((self.thresholds.drive_crisis - current) / trend * 50.0) as u32,
                        trend_direction: TrendDirection::Increasing,
                    });
                }
            }
        }

        None
    }

    fn predict_exploration_decline(&self, metrics: &SimulationMetrics) -> Option<PatternPrediction> {
        let len = metrics.snapshots.len();
        if len < 3 {
            return None;
        }

        let recent = &metrics.snapshots[len - 3..];
        let efficiencies: Vec<f32> = recent.iter()
            .map(|s| s.curiosity.average_exploration_efficiency)
            .collect();

        if efficiencies.len() >= 2 {
            let trend = efficiencies.last().unwrap() - efficiencies.first().unwrap();
            let current = *efficiencies.last().unwrap();

            if trend < -0.1 && current < 0.5 {
                return Some(PatternPrediction {
                    predicted_pattern: "KnowledgeSaturation".to_string(),
                    confidence: (trend.abs() * 2.0).min(0.8),
                    estimated_ticks_until: (current / trend.abs() * 50.0) as u32,
                    trend_direction: TrendDirection::Decreasing,
                });
            }
        }

        None
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

    fn detect_mass_migration(&mut self, metrics: &SimulationMetrics, current_tick: u32) {
        let len = metrics.snapshots.len();
        if len < 3 {
            return;
        }

        let recent = &metrics.snapshots[len - 3..];
        let total_abandonments: u32 = recent.iter()
            .map(|s| s.population.abandonments_this_tick)
            .sum();

        let avg_pop: f32 = recent.iter()
            .map(|s| s.population.total as f32)
            .sum::<f32>() / recent.len() as f32;

        if avg_pop == 0.0 {
            return;
        }

        let abandonment_rate = total_abandonments as f32 / avg_pop;

        // Mass migration: >15% abandonment rate
        if abandonment_rate > self.thresholds.mass_migration {
            let severity = ((abandonment_rate - self.thresholds.mass_migration) / 0.35).min(1.0);

            if severity >= self.detection_threshold {
                self.report_pattern(EmergentPattern {
                    pattern_type: PatternType::MassMigration { abandonment_rate },
                    detected_at_tick: current_tick,
                    severity,
                    description: format!(
                        "Mass migration: {:.1}% of agents abandoned society ({} agents left)",
                        abandonment_rate * 100.0,
                        total_abandonments
                    ),
                });
            }
        }
    }

    fn detect_curiosity_patterns(&mut self, metrics: &SimulationMetrics, current_tick: u32) {
        if let Some(last_snapshot) = metrics.snapshots.last() {
            let curiosity = &last_snapshot.curiosity;
            let total_pop = last_snapshot.population.total;

            if total_pop == 0 {
                return;
            }

            // Detect discovery boom
            if let Some(prev_discoveries) = self.previous_discoveries {
                let current_discoveries = curiosity.total_discoveries;
                if prev_discoveries > 0 {
                    let discovery_rate = current_discoveries as f32 / prev_discoveries as f32;

                    if discovery_rate > self.thresholds.discovery_boom {
                        let severity = ((discovery_rate - self.thresholds.discovery_boom) / 3.0).min(1.0);

                        if severity >= self.detection_threshold {
                            // Find most common discovery type
                            let top_type = curiosity.discoveries_by_type.iter()
                                .max_by_key(|(_, &count)| count)
                                .map(|(t, _)| t.clone());

                            self.report_pattern(EmergentPattern {
                                pattern_type: PatternType::DiscoveryBoom {
                                    discovery_rate,
                                    discovery_type: top_type.clone(),
                                },
                                detected_at_tick: current_tick,
                                severity,
                                description: format!(
                                    "Discovery boom: {:.1}x increase in discoveries{}",
                                    discovery_rate,
                                    top_type.map(|t| format!(" (primarily {})", t)).unwrap_or_default()
                                ),
                            });
                        }
                    }
                }
            }
            self.previous_discoveries = Some(curiosity.total_discoveries);

            // Detect exploration surge
            if let Some(prev_explorations) = self.previous_explorations {
                let current_explorations = curiosity.total_curiosity_driven_explorations;
                if prev_explorations > 0 {
                    let exploration_rate = (current_explorations - prev_explorations) as f32 / total_pop as f32;

                    if exploration_rate > self.thresholds.exploration_surge {
                        let severity = ((exploration_rate - self.thresholds.exploration_surge) / 1.0).min(1.0);

                        if severity >= self.detection_threshold {
                            self.report_pattern(EmergentPattern {
                                pattern_type: PatternType::ExplorationSurge { exploration_rate },
                                detected_at_tick: current_tick,
                                severity,
                                description: format!(
                                    "Exploration surge: {:.1} new explorations per agent",
                                    exploration_rate
                                ),
                            });
                        }
                    }

                    // Detect exploration decline
                    if exploration_rate < -0.3 {
                        let decline_rate = exploration_rate.abs();
                        let severity = (decline_rate / 0.7).min(1.0);

                        if severity >= self.detection_threshold {
                            self.report_pattern(EmergentPattern {
                                pattern_type: PatternType::ExplorationDecline { decline_rate },
                                detected_at_tick: current_tick,
                                severity,
                                description: format!(
                                    "Exploration decline: {:.1}% drop in exploration activity",
                                    decline_rate * 100.0
                                ),
                            });
                        }
                    }
                }
            }
            self.previous_explorations = Some(curiosity.total_curiosity_driven_explorations);

            // Detect curiosity awakening (many agents with high curiosity)
            let high_curiosity_rate = curiosity.agents_with_high_curiosity as f32 / total_pop as f32;
            if high_curiosity_rate > self.thresholds.curiosity_awakening {
                let severity = ((high_curiosity_rate - self.thresholds.curiosity_awakening) / 0.6).min(1.0);

                if severity >= self.detection_threshold {
                    self.report_pattern(EmergentPattern {
                        pattern_type: PatternType::CuriosityAwakening { high_curiosity_rate },
                        detected_at_tick: current_tick,
                        severity,
                        description: format!(
                            "Curiosity awakening: {:.1}% of agents have high curiosity drive",
                            high_curiosity_rate * 100.0
                        ),
                    });
                }
            }

            // Detect knowledge saturation (declining exploration efficiency)
            if let Some(prev_efficiency) = self.previous_efficiency {
                let current_efficiency = curiosity.average_exploration_efficiency;
                let efficiency_drop = prev_efficiency - current_efficiency;

                if efficiency_drop > 0.2 && current_efficiency < 0.3 {
                    let severity = (efficiency_drop / 0.5).min(1.0);

                    if severity >= self.detection_threshold {
                        self.report_pattern(EmergentPattern {
                            pattern_type: PatternType::KnowledgeSaturation { efficiency_drop },
                            detected_at_tick: current_tick,
                            severity,
                            description: format!(
                                "Knowledge saturation: Exploration efficiency dropped by {:.1}% (now {:.1}%)",
                                efficiency_drop * 100.0,
                                current_efficiency * 100.0
                            ),
                        });
                    }
                }
            }
            self.previous_efficiency = Some(curiosity.average_exploration_efficiency);
        }
    }

    fn detect_compound_crises(&mut self, metrics: &SimulationMetrics, current_tick: u32) {
        if let Some(last_snapshot) = metrics.snapshots.last() {
            let total_pop = last_snapshot.population.total;
            if total_pop == 0 {
                return;
            }

            // Find all drives in crisis state
            let mut crisis_drives: Vec<DriveType> = Vec::new();
            let mut total_crisis_severity = 0.0;

            for (drive_type, drive_snapshot) in &last_snapshot.drives {
                let critical_percentage = drive_snapshot.critical_count as f32 / total_pop as f32;

                if critical_percentage > 0.2 { // Lower threshold for compound detection
                    crisis_drives.push(drive_type.clone());
                    total_crisis_severity += critical_percentage;
                }
            }

            // Compound crisis: 2+ drives in crisis simultaneously
            if crisis_drives.len() >= 2 {
                let severity = (total_crisis_severity / crisis_drives.len() as f32).min(1.0);

                if severity >= self.detection_threshold {
                    let drive_names: Vec<String> = crisis_drives.iter()
                        .map(|d| format!("{:?}", d))
                        .collect();

                    self.report_pattern(EmergentPattern {
                        pattern_type: PatternType::CompoundCrisis {
                            crisis_types: crisis_drives.clone(),
                            severity,
                        },
                        detected_at_tick: current_tick,
                        severity,
                        description: format!(
                            "Compound crisis: {} drives in crisis ({}) - society destabilizing",
                            crisis_drives.len(),
                            drive_names.join(", ")
                        ),
                    });
                }
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
        patterns.sort_by(|a, b| b.severity.partial_cmp(&a.severity).unwrap_or(std::cmp::Ordering::Equal));
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
    fn test_detector_with_custom_thresholds() {
        let thresholds = DetectionThresholds {
            trait_clustering: 0.70,
            population_boom: 0.60,
            ..Default::default()
        };
        let detector = EmergenceDetector::with_thresholds(thresholds.clone());
        assert_eq!(detector.thresholds.trait_clustering, 0.70);
        assert_eq!(detector.thresholds.population_boom, 0.60);
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

    #[test]
    fn test_training_sample_addition() {
        let mut detector = EmergenceDetector::new();
        assert!(detector.training_samples.is_empty());

        let sample = TrainingSample {
            pattern_type: "population_boom".to_string(),
            metric_values: {
                let mut m = HashMap::new();
                m.insert("growth_rate".to_string(), 0.75);
                m
            },
            is_positive: true,
            rated_severity: 0.8,
        };

        detector.add_training_sample(sample);
        assert_eq!(detector.training_samples.len(), 1);
    }

    #[test]
    fn test_calibration_without_samples() {
        let mut detector = EmergenceDetector::new();
        let result = detector.calibrate_from_training();
        assert!(!result.success);
        assert!(result.adjustments.is_empty());
    }

    #[test]
    fn test_calibration_with_samples() {
        let mut detector = EmergenceDetector::new();

        // Add positive samples
        for growth in [0.6, 0.7, 0.8] {
            detector.add_training_sample(TrainingSample {
                pattern_type: "population_boom".to_string(),
                metric_values: {
                    let mut m = HashMap::new();
                    m.insert("growth_rate".to_string(), growth);
                    m
                },
                is_positive: true,
                rated_severity: 0.8,
            });
        }

        // Add negative samples
        for growth in [0.2, 0.3, 0.4] {
            detector.add_training_sample(TrainingSample {
                pattern_type: "population_boom".to_string(),
                metric_values: {
                    let mut m = HashMap::new();
                    m.insert("growth_rate".to_string(), growth);
                    m
                },
                is_positive: false,
                rated_severity: 0.2,
            });
        }

        let result = detector.calibrate_from_training();
        assert!(result.success);
        assert_eq!(result.message, "Calibrated from 6 training samples");
    }

    #[test]
    fn test_pattern_prediction_with_insufficient_data() {
        let detector = EmergenceDetector::new();
        let metrics = SimulationMetrics::new(1, 100);

        let predictions = detector.predict_patterns(&metrics);
        assert!(predictions.is_empty());
    }

    #[test]
    fn test_new_pattern_types() {
        // Test that new pattern types can be created
        let discovery_boom = PatternType::DiscoveryBoom {
            discovery_rate: 3.0,
            discovery_type: Some("storage".to_string()),
        };

        let exploration_surge = PatternType::ExplorationSurge {
            exploration_rate: 0.8,
        };

        let curiosity_awakening = PatternType::CuriosityAwakening {
            high_curiosity_rate: 0.55,
        };

        let compound_crisis = PatternType::CompoundCrisis {
            crisis_types: vec![DriveType::Hunger, DriveType::Thirst],
            severity: 0.7,
        };

        // Verify pattern discrimination works
        assert_ne!(
            std::mem::discriminant(&discovery_boom),
            std::mem::discriminant(&exploration_surge)
        );
        assert_ne!(
            std::mem::discriminant(&curiosity_awakening),
            std::mem::discriminant(&compound_crisis)
        );
    }

    #[test]
    fn test_detection_thresholds_default() {
        let thresholds = DetectionThresholds::default();

        assert_eq!(thresholds.trait_clustering, 0.60);
        assert_eq!(thresholds.population_boom, 0.50);
        assert_eq!(thresholds.population_collapse, 0.30);
        assert_eq!(thresholds.mass_migration, 0.15);
        assert_eq!(thresholds.discovery_boom, 2.0);
        assert_eq!(thresholds.exploration_surge, 0.50);
        assert_eq!(thresholds.curiosity_awakening, 0.40);
    }

    #[test]
    fn test_trend_direction() {
        assert_eq!(TrendDirection::Increasing, TrendDirection::Increasing);
        assert_ne!(TrendDirection::Increasing, TrendDirection::Decreasing);

        // Test serialization
        let direction = TrendDirection::Stable;
        let serialized = serde_json::to_string(&direction).unwrap();
        assert!(serialized.contains("Stable"));
    }

    #[test]
    fn test_pattern_prediction_structure() {
        let prediction = PatternPrediction {
            predicted_pattern: "PopulationBoom".to_string(),
            confidence: 0.75,
            estimated_ticks_until: 100,
            trend_direction: TrendDirection::Increasing,
        };

        assert_eq!(prediction.predicted_pattern, "PopulationBoom");
        assert_eq!(prediction.confidence, 0.75);
        assert_eq!(prediction.estimated_ticks_until, 100);
        assert_eq!(prediction.trend_direction, TrendDirection::Increasing);
    }

    #[test]
    fn test_calibration_result_structure() {
        let result = CalibrationResult {
            success: true,
            adjustments: {
                let mut m = HashMap::new();
                m.insert("threshold_a".to_string(), 0.55);
                m
            },
            message: "Success".to_string(),
        };

        assert!(result.success);
        assert_eq!(result.adjustments.get("threshold_a"), Some(&0.55));
    }
}
