// src/bevy_gui/resources/simulation_errors.rs
//! Error handling for simulation thread communication.

use bevy::prelude::*;

/// Severity level of a simulation error
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorSeverity {
    /// Informational warning that doesn't affect simulation
    Warning,
    /// Error that may affect simulation behavior
    Error,
    /// Fatal error that stops the simulation
    Fatal,
}

impl ErrorSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            ErrorSeverity::Warning => "Warning",
            ErrorSeverity::Error => "Error",
            ErrorSeverity::Fatal => "Fatal",
        }
    }
}

/// A single error from the simulation thread
#[derive(Debug, Clone)]
pub struct SimulationError {
    /// Simulation tick when the error occurred
    pub tick: u32,
    /// Error message
    pub message: String,
    /// Severity level
    pub severity: ErrorSeverity,
    /// Timestamp (Bevy time elapsed seconds)
    pub timestamp: f64,
    /// Optional context about what operation caused the error
    pub context: Option<String>,
}

impl SimulationError {
    pub fn warning(tick: u32, message: impl Into<String>) -> Self {
        Self {
            tick,
            message: message.into(),
            severity: ErrorSeverity::Warning,
            timestamp: 0.0,
            context: None,
        }
    }

    pub fn error(tick: u32, message: impl Into<String>) -> Self {
        Self {
            tick,
            message: message.into(),
            severity: ErrorSeverity::Error,
            timestamp: 0.0,
            context: None,
        }
    }

    pub fn fatal(tick: u32, message: impl Into<String>) -> Self {
        Self {
            tick,
            message: message.into(),
            severity: ErrorSeverity::Fatal,
            timestamp: 0.0,
            context: None,
        }
    }

    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context = Some(context.into());
        self
    }

    pub fn with_timestamp(mut self, timestamp: f64) -> Self {
        self.timestamp = timestamp;
        self
    }
}

/// Resource storing simulation errors for display in GUI
#[derive(Resource)]
pub struct SimulationErrors {
    /// List of errors (newest first)
    pub errors: Vec<SimulationError>,
    /// Maximum number of errors to retain
    pub max_errors: usize,
    /// Whether the simulation thread has reported a fatal error
    pub has_fatal: bool,
    /// Count of unacknowledged errors
    pub unacknowledged_count: usize,
}

impl Default for SimulationErrors {
    fn default() -> Self {
        Self {
            errors: Vec::with_capacity(100),
            max_errors: 100,
            has_fatal: false,
            unacknowledged_count: 0,
        }
    }
}

impl SimulationErrors {
    /// Add an error to the list
    pub fn push(&mut self, error: SimulationError) {
        if error.severity == ErrorSeverity::Fatal {
            self.has_fatal = true;
        }

        self.unacknowledged_count += 1;
        self.errors.insert(0, error);

        // Trim old errors
        while self.errors.len() > self.max_errors {
            self.errors.pop();
        }
    }

    /// Acknowledge all errors (mark as seen)
    pub fn acknowledge_all(&mut self) {
        self.unacknowledged_count = 0;
    }

    /// Clear all errors
    pub fn clear(&mut self) {
        self.errors.clear();
        self.has_fatal = false;
        self.unacknowledged_count = 0;
    }

    /// Get errors filtered by severity
    pub fn errors_by_severity(&self, severity: ErrorSeverity) -> impl Iterator<Item = &SimulationError> {
        self.errors.iter().filter(move |e| e.severity == severity)
    }

    /// Get the most recent error
    pub fn latest(&self) -> Option<&SimulationError> {
        self.errors.first()
    }

    /// Check if there are any errors
    pub fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }

    /// Get count of errors
    pub fn len(&self) -> usize {
        self.errors.len()
    }

    /// Check if there are unacknowledged errors
    pub fn has_unacknowledged(&self) -> bool {
        self.unacknowledged_count > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simulation_error_creation() {
        let err = SimulationError::warning(100, "Test warning");
        assert_eq!(err.tick, 100);
        assert_eq!(err.message, "Test warning");
        assert_eq!(err.severity, ErrorSeverity::Warning);
        assert!(err.context.is_none());
    }

    #[test]
    fn test_simulation_error_with_context() {
        let err = SimulationError::error(50, "Test error")
            .with_context("During save operation");
        assert_eq!(err.context, Some("During save operation".to_string()));
    }

    #[test]
    fn test_simulation_errors_push() {
        let mut errors = SimulationErrors::default();
        assert!(errors.is_empty());

        errors.push(SimulationError::warning(1, "Warning 1"));
        assert_eq!(errors.len(), 1);
        assert_eq!(errors.unacknowledged_count, 1);

        errors.push(SimulationError::error(2, "Error 1"));
        assert_eq!(errors.len(), 2);
        assert_eq!(errors.unacknowledged_count, 2);

        // Newest should be first
        assert_eq!(errors.latest().unwrap().tick, 2);
    }

    #[test]
    fn test_simulation_errors_fatal_flag() {
        let mut errors = SimulationErrors::default();
        assert!(!errors.has_fatal);

        errors.push(SimulationError::warning(1, "Warning"));
        assert!(!errors.has_fatal);

        errors.push(SimulationError::fatal(2, "Fatal error"));
        assert!(errors.has_fatal);
    }

    #[test]
    fn test_simulation_errors_acknowledge() {
        let mut errors = SimulationErrors::default();
        errors.push(SimulationError::warning(1, "Warning 1"));
        errors.push(SimulationError::warning(2, "Warning 2"));

        assert!(errors.has_unacknowledged());
        assert_eq!(errors.unacknowledged_count, 2);

        errors.acknowledge_all();
        assert!(!errors.has_unacknowledged());
        assert_eq!(errors.unacknowledged_count, 0);
        assert_eq!(errors.len(), 2); // Errors still exist
    }

    #[test]
    fn test_simulation_errors_max_capacity() {
        let mut errors = SimulationErrors::default();
        errors.max_errors = 5;

        for i in 0..10 {
            errors.push(SimulationError::warning(i, format!("Warning {}", i)));
        }

        assert_eq!(errors.len(), 5);
        // Most recent should be tick 9
        assert_eq!(errors.latest().unwrap().tick, 9);
    }
}
