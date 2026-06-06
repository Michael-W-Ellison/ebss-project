// src/core/errors.rs
//! Error handling and recovery for simulation operations.
//!
//! Provides structured error types for simulation failures and
//! recovery strategies to keep the simulation running even when
//! individual operations fail.

use std::fmt;
use uuid::Uuid;

/// Result type for simulation operations
pub type SimulationResult<T> = Result<T, SimulationError>;

/// Categories of simulation errors
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorCategory {
    /// Error during agent behavior processing
    AgentProcessing,
    /// Error during action execution
    ActionExecution,
    /// Error during world state update
    WorldUpdate,
    /// Error during population lifecycle
    PopulationLifecycle,
    /// Error during save/load operations
    Persistence,
    /// Error during resource management
    ResourceManagement,
    /// Configuration error
    Configuration,
    /// Internal error (should not happen in normal operation)
    Internal,
}

impl fmt::Display for ErrorCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorCategory::AgentProcessing => write!(f, "Agent Processing"),
            ErrorCategory::ActionExecution => write!(f, "Action Execution"),
            ErrorCategory::WorldUpdate => write!(f, "World Update"),
            ErrorCategory::PopulationLifecycle => write!(f, "Population Lifecycle"),
            ErrorCategory::Persistence => write!(f, "Save/Load"),
            ErrorCategory::ResourceManagement => write!(f, "Resource Management"),
            ErrorCategory::Configuration => write!(f, "Configuration"),
            ErrorCategory::Internal => write!(f, "Internal"),
        }
    }
}

/// Severity level of errors
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ErrorSeverity {
    /// Warning - operation completed but with issues
    Warning,
    /// Error - operation failed but simulation can continue
    Recoverable,
    /// Critical - serious error but simulation might continue
    Critical,
    /// Fatal - simulation must stop
    Fatal,
}

impl fmt::Display for ErrorSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorSeverity::Warning => write!(f, "WARN"),
            ErrorSeverity::Recoverable => write!(f, "ERROR"),
            ErrorSeverity::Critical => write!(f, "CRITICAL"),
            ErrorSeverity::Fatal => write!(f, "FATAL"),
        }
    }
}

/// Structured error for simulation operations
#[derive(Debug, Clone)]
pub struct SimulationError {
    /// Error category
    pub category: ErrorCategory,
    /// Severity level
    pub severity: ErrorSeverity,
    /// Human-readable message
    pub message: String,
    /// Tick when error occurred
    pub tick: u32,
    /// Agent involved (if applicable)
    pub agent_id: Option<Uuid>,
    /// Recovery action taken
    pub recovery: Option<RecoveryAction>,
    /// Additional context
    pub context: Option<String>,
}

impl SimulationError {
    /// Create a new simulation error
    pub fn new(category: ErrorCategory, severity: ErrorSeverity, message: impl Into<String>) -> Self {
        Self {
            category,
            severity,
            message: message.into(),
            tick: 0,
            agent_id: None,
            recovery: None,
            context: None,
        }
    }

    /// Create a warning
    pub fn warning(category: ErrorCategory, message: impl Into<String>) -> Self {
        Self::new(category, ErrorSeverity::Warning, message)
    }

    /// Create a recoverable error
    pub fn recoverable(category: ErrorCategory, message: impl Into<String>) -> Self {
        Self::new(category, ErrorSeverity::Recoverable, message)
    }

    /// Create a critical error
    pub fn critical(category: ErrorCategory, message: impl Into<String>) -> Self {
        Self::new(category, ErrorSeverity::Critical, message)
    }

    /// Create a fatal error
    pub fn fatal(category: ErrorCategory, message: impl Into<String>) -> Self {
        Self::new(category, ErrorSeverity::Fatal, message)
    }

    /// Set the tick when error occurred
    pub fn at_tick(mut self, tick: u32) -> Self {
        self.tick = tick;
        self
    }

    /// Set the agent involved
    pub fn for_agent(mut self, agent_id: Uuid) -> Self {
        self.agent_id = Some(agent_id);
        self
    }

    /// Set recovery action taken
    pub fn with_recovery(mut self, recovery: RecoveryAction) -> Self {
        self.recovery = Some(recovery);
        self
    }

    /// Add context information
    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context = Some(context.into());
        self
    }

    /// Check if this error is recoverable
    pub fn is_recoverable(&self) -> bool {
        matches!(self.severity, ErrorSeverity::Warning | ErrorSeverity::Recoverable)
    }

    /// Check if this error is fatal
    pub fn is_fatal(&self) -> bool {
        matches!(self.severity, ErrorSeverity::Fatal)
    }
}

impl fmt::Display for SimulationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{} {}] tick={}", self.severity, self.category, self.tick)?;

        if let Some(agent_id) = self.agent_id {
            write!(f, " agent={}", &agent_id.to_string()[..8])?;
        }

        write!(f, " {}", self.message)?;

        if let Some(ref ctx) = self.context {
            write!(f, " ({})", ctx)?;
        }

        if let Some(ref recovery) = self.recovery {
            write!(f, " [recovered: {}]", recovery)?;
        }

        Ok(())
    }
}

impl std::error::Error for SimulationError {}

/// Recovery actions that can be taken after an error
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryAction {
    /// Skip the current operation and continue
    SkipOperation,
    /// Skip processing this agent for this tick
    SkipAgent,
    /// Reset agent to safe state
    ResetAgentState,
    /// Remove agent from simulation
    RemoveAgent,
    /// Rollback to last checkpoint
    RollbackCheckpoint,
    /// No recovery possible
    None,
    /// Custom recovery action
    Custom(String),
}

impl fmt::Display for RecoveryAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RecoveryAction::SkipOperation => write!(f, "skipped operation"),
            RecoveryAction::SkipAgent => write!(f, "skipped agent"),
            RecoveryAction::ResetAgentState => write!(f, "reset agent state"),
            RecoveryAction::RemoveAgent => write!(f, "removed agent"),
            RecoveryAction::RollbackCheckpoint => write!(f, "rolled back to checkpoint"),
            RecoveryAction::None => write!(f, "no recovery"),
            RecoveryAction::Custom(action) => write!(f, "{}", action),
        }
    }
}

/// Error collector for accumulating errors during a tick
#[derive(Debug, Default)]
pub struct TickErrorCollector {
    /// Collected errors
    errors: Vec<SimulationError>,
    /// Current tick
    tick: u32,
    /// Whether a fatal error has occurred
    has_fatal: bool,
}

impl TickErrorCollector {
    /// Create a new error collector for a tick
    pub fn new(tick: u32) -> Self {
        Self {
            errors: Vec::new(),
            tick,
            has_fatal: false,
        }
    }

    /// Record an error
    pub fn record(&mut self, mut error: SimulationError) {
        error.tick = self.tick;
        if error.is_fatal() {
            self.has_fatal = true;
        }
        log::warn!("{}", error);
        self.errors.push(error);
    }

    /// Record a warning
    pub fn warn(&mut self, category: ErrorCategory, message: impl Into<String>) {
        self.record(SimulationError::warning(category, message));
    }

    /// Record a recoverable error with automatic recovery
    pub fn recover(&mut self, category: ErrorCategory, message: impl Into<String>, recovery: RecoveryAction) {
        self.record(
            SimulationError::recoverable(category, message)
                .with_recovery(recovery)
        );
    }

    /// Check if any fatal errors occurred
    pub fn has_fatal(&self) -> bool {
        self.has_fatal
    }

    /// Check if any errors occurred
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Get error count
    pub fn error_count(&self) -> usize {
        self.errors.len()
    }

    /// Get all errors
    pub fn errors(&self) -> &[SimulationError] {
        &self.errors
    }

    /// Take all errors
    pub fn take_errors(self) -> Vec<SimulationError> {
        self.errors
    }
}

/// Trait for safe execution with error recovery
pub trait SafeExecute<T> {
    /// Execute the operation safely, returning a default on error
    fn safe_execute_or(self, default: T, collector: &mut TickErrorCollector, category: ErrorCategory, context: &str) -> T;

    /// Execute the operation safely, returning None on error
    fn safe_execute(self, collector: &mut TickErrorCollector, category: ErrorCategory, context: &str) -> Option<T>;
}

impl<T, E: std::fmt::Debug> SafeExecute<T> for Result<T, E> {
    fn safe_execute_or(self, default: T, collector: &mut TickErrorCollector, category: ErrorCategory, context: &str) -> T {
        match self {
            Ok(value) => value,
            Err(e) => {
                collector.recover(
                    category,
                    format!("{:?}", e),
                    RecoveryAction::SkipOperation,
                );
                if !context.is_empty() {
                    if let Some(last) = collector.errors.last_mut() {
                        last.context = Some(context.to_string());
                    }
                }
                default
            }
        }
    }

    fn safe_execute(self, collector: &mut TickErrorCollector, category: ErrorCategory, context: &str) -> Option<T> {
        match self {
            Ok(value) => Some(value),
            Err(e) => {
                collector.recover(
                    category,
                    format!("{:?}", e),
                    RecoveryAction::SkipOperation,
                );
                if !context.is_empty() {
                    if let Some(last) = collector.errors.last_mut() {
                        last.context = Some(context.to_string());
                    }
                }
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let error = SimulationError::recoverable(
            ErrorCategory::AgentProcessing,
            "Agent failed to process action"
        )
        .at_tick(100)
        .for_agent(Uuid::new_v4())
        .with_context("During hunger satisfaction");

        assert_eq!(error.category, ErrorCategory::AgentProcessing);
        assert_eq!(error.severity, ErrorSeverity::Recoverable);
        assert_eq!(error.tick, 100);
        assert!(error.agent_id.is_some());
        assert!(error.is_recoverable());
        assert!(!error.is_fatal());
    }

    #[test]
    fn test_fatal_error() {
        let error = SimulationError::fatal(
            ErrorCategory::Internal,
            "Critical system failure"
        );

        assert!(error.is_fatal());
        assert!(!error.is_recoverable());
    }

    #[test]
    fn test_error_collector() {
        let mut collector = TickErrorCollector::new(50);

        assert!(!collector.has_errors());
        assert!(!collector.has_fatal());

        collector.warn(ErrorCategory::AgentProcessing, "Minor issue");
        assert!(collector.has_errors());
        assert!(!collector.has_fatal());
        assert_eq!(collector.error_count(), 1);

        collector.recover(
            ErrorCategory::ActionExecution,
            "Action failed",
            RecoveryAction::SkipOperation
        );
        assert_eq!(collector.error_count(), 2);
    }

    #[test]
    fn test_error_collector_fatal() {
        let mut collector = TickErrorCollector::new(50);

        collector.record(SimulationError::fatal(ErrorCategory::Internal, "Fatal"));
        assert!(collector.has_fatal());
    }

    #[test]
    fn test_safe_execute_ok() {
        let mut collector = TickErrorCollector::new(0);
        let result: Result<i32, &str> = Ok(42);

        let value = result.safe_execute_or(0, &mut collector, ErrorCategory::Internal, "");
        assert_eq!(value, 42);
        assert!(!collector.has_errors());
    }

    #[test]
    fn test_safe_execute_err() {
        let mut collector = TickErrorCollector::new(0);
        let result: Result<i32, &str> = Err("something went wrong");

        let value = result.safe_execute_or(0, &mut collector, ErrorCategory::Internal, "test context");
        assert_eq!(value, 0);
        assert!(collector.has_errors());
        assert_eq!(collector.error_count(), 1);
    }

    #[test]
    fn test_error_display() {
        let error = SimulationError::recoverable(
            ErrorCategory::AgentProcessing,
            "Test error"
        )
        .at_tick(100)
        .with_recovery(RecoveryAction::SkipAgent);

        let display = format!("{}", error);
        assert!(display.contains("ERROR"));
        assert!(display.contains("Agent Processing"));
        assert!(display.contains("tick=100"));
        assert!(display.contains("Test error"));
        assert!(display.contains("skipped agent"));
    }
}
