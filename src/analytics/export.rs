// src/analytics/export.rs
//! Data export for visualization and external analysis.

use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Write;
use std::path::Path;

use crate::analytics::metrics::SimulationMetrics;
use crate::analytics::emergence::EmergenceDetector;

/// Supported export formats
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExportFormat {
    Json,
    JsonPretty,
    Csv,
}

/// Data exporter for metrics and emergence data
#[derive(Debug)]
pub struct DataExporter;

impl DataExporter {
    /// Export simulation metrics to a file
    pub fn export_metrics(
        metrics: &SimulationMetrics,
        path: impl AsRef<Path>,
        format: ExportFormat,
    ) -> std::io::Result<()> {
        match format {
            ExportFormat::Json => Self::export_metrics_json(metrics, path, false),
            ExportFormat::JsonPretty => Self::export_metrics_json(metrics, path, true),
            ExportFormat::Csv => Self::export_metrics_csv(metrics, path),
        }
    }

    fn export_metrics_json(
        metrics: &SimulationMetrics,
        path: impl AsRef<Path>,
        pretty: bool,
    ) -> std::io::Result<()> {
        let mut file = File::create(path)?;

        let json = if pretty {
            serde_json::to_string_pretty(metrics).map_err(|e| {
                std::io::Error::other(e)
            })?
        } else {
            serde_json::to_string(metrics).map_err(|e| {
                std::io::Error::other(e)
            })?
        };

        file.write_all(json.as_bytes())?;
        Ok(())
    }

    fn export_metrics_csv(
        metrics: &SimulationMetrics,
        path: impl AsRef<Path>,
    ) -> std::io::Result<()> {
        let mut file = File::create(path)?;

        // Write header
        writeln!(
            file,
            "tick,population,average_happiness,average_age,births,deaths,abandonments,total_relationships,average_trust,average_affection,family_bonds,conflicts"
        )?;

        // Write data rows
        for snapshot in &metrics.snapshots {
            writeln!(
                file,
                "{},{},{:.3},{:.1},{},{},{},{},{:.3},{:.3},{},{}",
                snapshot.tick,
                snapshot.population.total,
                snapshot.population.average_happiness,
                snapshot.population.average_age,
                snapshot.population.births_this_tick,
                snapshot.population.deaths_this_tick,
                snapshot.population.abandonments_this_tick,
                snapshot.relationships.total_relationships,
                snapshot.relationships.average_trust,
                snapshot.relationships.average_affection,
                snapshot.relationships.family_bonds,
                snapshot.relationships.conflicts,
            )?;
        }

        Ok(())
    }

    /// Export emergence patterns to a file
    pub fn export_emergence(
        detector: &EmergenceDetector,
        path: impl AsRef<Path>,
        format: ExportFormat,
    ) -> std::io::Result<()> {
        match format {
            ExportFormat::Json => Self::export_emergence_json(detector, path, false),
            ExportFormat::JsonPretty => Self::export_emergence_json(detector, path, true),
            ExportFormat::Csv => Self::export_emergence_csv(detector, path),
        }
    }

    fn export_emergence_json(
        detector: &EmergenceDetector,
        path: impl AsRef<Path>,
        pretty: bool,
    ) -> std::io::Result<()> {
        let mut file = File::create(path)?;

        let json = if pretty {
            serde_json::to_string_pretty(&detector.detected_patterns).map_err(|e| {
                std::io::Error::other(e)
            })?
        } else {
            serde_json::to_string(&detector.detected_patterns).map_err(|e| {
                std::io::Error::other(e)
            })?
        };

        file.write_all(json.as_bytes())?;
        Ok(())
    }

    fn export_emergence_csv(
        detector: &EmergenceDetector,
        path: impl AsRef<Path>,
    ) -> std::io::Result<()> {
        let mut file = File::create(path)?;

        // Write header
        writeln!(file, "tick,pattern_type,severity,description")?;

        // Write data rows
        for pattern in &detector.detected_patterns {
            let pattern_type_str = format!("{:?}", pattern.pattern_type);
            writeln!(
                file,
                "{},{},{:.3},\"{}\"",
                pattern.detected_at_tick, pattern_type_str, pattern.severity, pattern.description
            )?;
        }

        Ok(())
    }

    /// Export population trend as simple CSV for graphing
    pub fn export_population_trend(
        metrics: &SimulationMetrics,
        path: impl AsRef<Path>,
    ) -> std::io::Result<()> {
        let mut file = File::create(path)?;

        writeln!(file, "tick,population")?;

        for (tick, pop) in metrics.population_trend() {
            writeln!(file, "{},{}", tick, pop)?;
        }

        Ok(())
    }

    /// Export happiness trend as simple CSV for graphing
    pub fn export_happiness_trend(
        metrics: &SimulationMetrics,
        path: impl AsRef<Path>,
    ) -> std::io::Result<()> {
        let mut file = File::create(path)?;

        writeln!(file, "tick,happiness")?;

        for (tick, happiness) in metrics.happiness_trend() {
            writeln!(file, "{},{:.3}", tick, happiness)?;
        }

        Ok(())
    }

    /// Export trait distribution over time
    pub fn export_trait_trends(
        metrics: &SimulationMetrics,
        path: impl AsRef<Path>,
    ) -> std::io::Result<()> {
        let mut file = File::create(path)?;

        // Collect all traits that appear in any snapshot
        let mut all_traits = std::collections::HashSet::new();
        for snapshot in &metrics.snapshots {
            for trait_item in snapshot.traits.keys() {
                all_traits.insert(trait_item.clone());
            }
        }

        let mut trait_vec: Vec<_> = all_traits.into_iter().collect();
        trait_vec.sort_by_key(|t| format!("{:?}", t));

        // Write header
        write!(file, "tick")?;
        for trait_item in &trait_vec {
            write!(file, ",{:?}", trait_item)?;
        }
        writeln!(file)?;

        // Write data rows
        for snapshot in &metrics.snapshots {
            write!(file, "{}", snapshot.tick)?;
            for trait_item in &trait_vec {
                let count = snapshot.traits.get(trait_item).copied().unwrap_or(0);
                write!(file, ",{}", count)?;
            }
            writeln!(file)?;
        }

        Ok(())
    }

    /// Export all data in a convenient format for visualization dashboard
    pub fn export_dashboard_data(
        metrics: &SimulationMetrics,
        emergence: &EmergenceDetector,
        output_dir: impl AsRef<Path>,
    ) -> std::io::Result<()> {
        let dir = output_dir.as_ref();
        std::fs::create_dir_all(dir)?;

        // Export metrics
        Self::export_metrics(metrics, dir.join("metrics.json"), ExportFormat::JsonPretty)?;
        Self::export_metrics(metrics, dir.join("metrics.csv"), ExportFormat::Csv)?;

        // Export emergence
        Self::export_emergence(emergence, dir.join("emergence.json"), ExportFormat::JsonPretty)?;
        Self::export_emergence(emergence, dir.join("emergence.csv"), ExportFormat::Csv)?;

        // Export trends
        Self::export_population_trend(metrics, dir.join("population_trend.csv"))?;
        Self::export_happiness_trend(metrics, dir.join("happiness_trend.csv"))?;
        Self::export_trait_trends(metrics, dir.join("trait_trends.csv"))?;

        // Export summary
        let summary = metrics.summary();
        let summary_json = serde_json::to_string_pretty(&summary)
            .map_err(|e| std::io::Error::other(e))?;
        let mut summary_file = File::create(dir.join("summary.json"))?;
        summary_file.write_all(summary_json.as_bytes())?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analytics::metrics::SimulationMetrics;
    use crate::analytics::emergence::EmergenceDetector;
    use crate::agents::Population;
    use tempfile::TempDir;

    #[test]
    fn test_export_metrics_json() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("metrics.json");

        let mut metrics = SimulationMetrics::new(1, 100);
        let population = Population::new();
        metrics.record_snapshot(0, &population);

        DataExporter::export_metrics(&metrics, &file_path, ExportFormat::Json).unwrap();

        assert!(file_path.exists());
    }

    #[test]
    fn test_export_metrics_csv() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("metrics.csv");

        let mut metrics = SimulationMetrics::new(1, 100);
        let population = Population::new();
        metrics.record_snapshot(0, &population);

        DataExporter::export_metrics(&metrics, &file_path, ExportFormat::Csv).unwrap();

        assert!(file_path.exists());

        let content = std::fs::read_to_string(&file_path).unwrap();
        assert!(content.contains("tick,population"));
    }

    #[test]
    fn test_export_emergence() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("emergence.json");

        let detector = EmergenceDetector::new();

        DataExporter::export_emergence(&detector, &file_path, ExportFormat::JsonPretty).unwrap();

        assert!(file_path.exists());
    }

    #[test]
    fn test_export_dashboard_data() {
        let temp_dir = TempDir::new().unwrap();

        let mut metrics = SimulationMetrics::new(1, 100);
        let population = Population::new();
        metrics.record_snapshot(0, &population);

        let detector = EmergenceDetector::new();

        DataExporter::export_dashboard_data(&metrics, &detector, temp_dir.path()).unwrap();

        assert!(temp_dir.path().join("metrics.json").exists());
        assert!(temp_dir.path().join("metrics.csv").exists());
        assert!(temp_dir.path().join("emergence.json").exists());
        assert!(temp_dir.path().join("summary.json").exists());
    }
}
