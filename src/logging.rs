// src/logging.rs
//! Structured logging system for simulation events and debugging.
//!
//! Provides context-aware logging that includes simulation state (tick, agent count)
//! in all log messages for easier debugging and analysis.
//!
//! # Usage
//!
//! ```ignore
//! use crate::logging::{SimulationContext, LogLevel};
//!
//! let ctx = SimulationContext::new(current_tick, agent_count);
//! sim_log!(ctx, LogLevel::Info, "Simulation started");
//! sim_log!(ctx, LogLevel::Debug, "Processing agent {}", agent_id);
//! ```

use std::fmt;

/// Log levels matching the standard log crate
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LogLevel::Error => write!(f, "ERROR"),
            LogLevel::Warn => write!(f, "WARN"),
            LogLevel::Info => write!(f, "INFO"),
            LogLevel::Debug => write!(f, "DEBUG"),
            LogLevel::Trace => write!(f, "TRACE"),
        }
    }
}

/// Context for structured simulation logging.
/// Includes tick number and agent count for correlation.
#[derive(Debug, Clone, Copy)]
pub struct SimulationContext {
    /// Current simulation tick
    pub tick: u32,
    /// Number of living agents
    pub agent_count: usize,
    /// Optional agent ID being processed
    pub current_agent: Option<uuid::Uuid>,
}

impl SimulationContext {
    /// Create a new simulation context
    pub fn new(tick: u32, agent_count: usize) -> Self {
        Self {
            tick,
            agent_count,
            current_agent: None,
        }
    }

    /// Create context with a specific agent
    pub fn with_agent(tick: u32, agent_count: usize, agent_id: uuid::Uuid) -> Self {
        Self {
            tick,
            agent_count,
            current_agent: Some(agent_id),
        }
    }

    /// Set the current agent being processed
    pub fn set_agent(&mut self, agent_id: uuid::Uuid) {
        self.current_agent = Some(agent_id);
    }

    /// Clear the current agent
    pub fn clear_agent(&mut self) {
        self.current_agent = None;
    }

    /// Format context as a prefix string
    pub fn prefix(&self) -> String {
        match self.current_agent {
            Some(id) => format!("[tick={} agents={} agent={}]", self.tick, self.agent_count, &id.to_string()[..8]),
            None => format!("[tick={} agents={}]", self.tick, self.agent_count),
        }
    }
}

impl Default for SimulationContext {
    fn default() -> Self {
        Self::new(0, 0)
    }
}

/// Event categories for filtering and analysis
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EventCategory {
    /// Simulation lifecycle (start, stop, tick)
    Simulation,
    /// Agent actions and decisions
    AgentAction,
    /// Combat and damage
    Combat,
    /// Resource gathering and crafting
    Economy,
    /// Social interactions
    Social,
    /// World events (weather, disasters)
    World,
    /// Performance metrics
    Performance,
    /// Errors and warnings
    Error,
}

impl fmt::Display for EventCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EventCategory::Simulation => write!(f, "SIM"),
            EventCategory::AgentAction => write!(f, "ACT"),
            EventCategory::Combat => write!(f, "CMB"),
            EventCategory::Economy => write!(f, "ECO"),
            EventCategory::Social => write!(f, "SOC"),
            EventCategory::World => write!(f, "WLD"),
            EventCategory::Performance => write!(f, "PRF"),
            EventCategory::Error => write!(f, "ERR"),
        }
    }
}

/// Structured log entry with full context
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub level: LogLevel,
    pub category: EventCategory,
    pub tick: u32,
    pub agent_count: usize,
    pub agent_id: Option<uuid::Uuid>,
    pub message: String,
}

impl LogEntry {
    /// Create a new log entry
    pub fn new(
        level: LogLevel,
        category: EventCategory,
        ctx: &SimulationContext,
        message: String,
    ) -> Self {
        Self {
            level,
            category,
            tick: ctx.tick,
            agent_count: ctx.agent_count,
            agent_id: ctx.current_agent,
            message,
        }
    }

    /// Format as a structured log line
    pub fn format(&self) -> String {
        let agent_str = self.agent_id
            .map(|id| format!(" agent={}", &id.to_string()[..8]))
            .unwrap_or_default();

        format!(
            "[{} {} tick={} pop={}{}] {}",
            self.level,
            self.category,
            self.tick,
            self.agent_count,
            agent_str,
            self.message
        )
    }
}

/// Log a message with simulation context using the standard log crate.
///
/// This macro provides structured logging with automatic context inclusion.
/// It delegates to the appropriate log macro (debug!, info!, warn!, error!)
/// based on the specified level.
#[macro_export]
macro_rules! sim_log {
    ($ctx:expr, $level:expr, $category:expr, $($arg:tt)*) => {{
        let entry = $crate::logging::LogEntry::new(
            $level,
            $category,
            $ctx,
            format!($($arg)*),
        );
        match $level {
            $crate::logging::LogLevel::Error => log::error!("{}", entry.format()),
            $crate::logging::LogLevel::Warn => log::warn!("{}", entry.format()),
            $crate::logging::LogLevel::Info => log::info!("{}", entry.format()),
            $crate::logging::LogLevel::Debug => log::debug!("{}", entry.format()),
            $crate::logging::LogLevel::Trace => log::trace!("{}", entry.format()),
        }
    }};
}

/// Convenience macro for simulation info logging
#[macro_export]
macro_rules! sim_info {
    ($ctx:expr, $category:expr, $($arg:tt)*) => {
        $crate::sim_log!($ctx, $crate::logging::LogLevel::Info, $category, $($arg)*)
    };
}

/// Convenience macro for simulation debug logging
#[macro_export]
macro_rules! sim_debug {
    ($ctx:expr, $category:expr, $($arg:tt)*) => {
        $crate::sim_log!($ctx, $crate::logging::LogLevel::Debug, $category, $($arg)*)
    };
}

/// Convenience macro for simulation warning logging
#[macro_export]
macro_rules! sim_warn {
    ($ctx:expr, $category:expr, $($arg:tt)*) => {
        $crate::sim_log!($ctx, $crate::logging::LogLevel::Warn, $category, $($arg)*)
    };
}

/// Convenience macro for simulation error logging
#[macro_export]
macro_rules! sim_error {
    ($ctx:expr, $category:expr, $($arg:tt)*) => {
        $crate::sim_log!($ctx, $crate::logging::LogLevel::Error, $category, $($arg)*)
    };
}

/// Initialize logging with default configuration.
/// Call this at application startup.
pub fn init_logging() {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info")
    )
    .format_timestamp_millis()
    .init();
}

/// Initialize logging with a specific filter level.
pub fn init_logging_with_level(level: &str) {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or(level)
    )
    .format_timestamp_millis()
    .init();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simulation_context_creation() {
        let ctx = SimulationContext::new(100, 50);
        assert_eq!(ctx.tick, 100);
        assert_eq!(ctx.agent_count, 50);
        assert!(ctx.current_agent.is_none());
    }

    #[test]
    fn test_simulation_context_with_agent() {
        let agent_id = uuid::Uuid::new_v4();
        let ctx = SimulationContext::with_agent(100, 50, agent_id);
        assert_eq!(ctx.current_agent, Some(agent_id));
    }

    #[test]
    fn test_context_prefix_without_agent() {
        let ctx = SimulationContext::new(100, 50);
        let prefix = ctx.prefix();
        assert!(prefix.contains("tick=100"));
        assert!(prefix.contains("agents=50"));
    }

    #[test]
    fn test_context_prefix_with_agent() {
        let agent_id = uuid::Uuid::new_v4();
        let ctx = SimulationContext::with_agent(100, 50, agent_id);
        let prefix = ctx.prefix();
        assert!(prefix.contains("agent="));
    }

    #[test]
    fn test_log_entry_format() {
        let ctx = SimulationContext::new(100, 50);
        let entry = LogEntry::new(
            LogLevel::Info,
            EventCategory::Simulation,
            &ctx,
            "Test message".to_string(),
        );
        let formatted = entry.format();
        assert!(formatted.contains("INFO"));
        assert!(formatted.contains("SIM"));
        assert!(formatted.contains("tick=100"));
        assert!(formatted.contains("Test message"));
    }

    #[test]
    fn test_event_category_display() {
        assert_eq!(format!("{}", EventCategory::Simulation), "SIM");
        assert_eq!(format!("{}", EventCategory::Combat), "CMB");
        assert_eq!(format!("{}", EventCategory::Economy), "ECO");
    }

    #[test]
    fn test_log_level_ordering() {
        assert!(LogLevel::Error < LogLevel::Warn);
        assert!(LogLevel::Warn < LogLevel::Info);
        assert!(LogLevel::Info < LogLevel::Debug);
        assert!(LogLevel::Debug < LogLevel::Trace);
    }
}
