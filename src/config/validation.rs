// src/config/validation.rs
//! Configuration validation types and error handling.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Trait for validating configuration structs.
pub trait ConfigValidation {
    /// Validate all fields and return an error if invalid.
    fn validate(&self) -> Result<(), ConfigError>;
}

/// Errors that can occur during configuration loading or validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConfigError {
    /// File I/O error
    IoError(String),

    /// TOML parsing error
    ParseError(String),

    /// TOML serialization error
    SerializeError(String),

    /// Value is out of acceptable range
    OutOfRange {
        field: String,
        value: f32,
        min: f32,
        max: f32,
    },

    /// Value is negative when it should be positive
    NegativeValue {
        field: String,
        value: f32,
    },

    /// Values are in wrong order (e.g., min > max)
    InvalidOrder {
        field: String,
        message: String,
    },

    /// Value is invalid for other reasons
    InvalidValue {
        field: String,
        message: String,
    },

    /// Required field is missing
    MissingField(String),

    /// Unknown field encountered
    UnknownField(String),

    /// Multiple validation errors
    Multiple(Vec<ConfigError>),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::IoError(msg) => write!(f, "I/O error: {}", msg),
            ConfigError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            ConfigError::SerializeError(msg) => write!(f, "Serialize error: {}", msg),
            ConfigError::OutOfRange { field, value, min, max } => {
                write!(f, "Value {} for '{}' is out of range [{}, {}]", value, field, min, max)
            }
            ConfigError::NegativeValue { field, value } => {
                write!(f, "Value {} for '{}' must be non-negative", value, field)
            }
            ConfigError::InvalidOrder { field, message } => {
                write!(f, "Invalid order in '{}': {}", field, message)
            }
            ConfigError::InvalidValue { field, message } => {
                write!(f, "Invalid value for '{}': {}", field, message)
            }
            ConfigError::MissingField(field) => {
                write!(f, "Missing required field: {}", field)
            }
            ConfigError::UnknownField(field) => {
                write!(f, "Unknown field: {}", field)
            }
            ConfigError::Multiple(errors) => {
                writeln!(f, "Multiple configuration errors:")?;
                for err in errors {
                    writeln!(f, "  - {}", err)?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for ConfigError {}

/// Builder for collecting multiple validation errors.
#[derive(Debug, Default)]
pub struct ValidationBuilder {
    errors: Vec<ConfigError>,
}

impl ValidationBuilder {
    /// Create a new validation builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an error to the collection.
    pub fn add_error(&mut self, error: ConfigError) {
        self.errors.push(error);
    }

    /// Check if there are any errors.
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Finish validation and return result.
    pub fn finish(self) -> Result<(), ConfigError> {
        match self.errors.len() {
            0 => Ok(()),
            1 => Err(self.errors.into_iter().next().unwrap()),
            _ => Err(ConfigError::Multiple(self.errors)),
        }
    }

    /// Validate a range constraint.
    pub fn validate_range(&mut self, field: &str, value: f32, min: f32, max: f32) {
        if !(min..=max).contains(&value) {
            self.add_error(ConfigError::OutOfRange {
                field: field.to_string(),
                value,
                min,
                max,
            });
        }
    }

    /// Validate a non-negative constraint.
    pub fn validate_non_negative(&mut self, field: &str, value: f32) {
        if value < 0.0 {
            self.add_error(ConfigError::NegativeValue {
                field: field.to_string(),
                value,
            });
        }
    }

    /// Validate a positive constraint.
    pub fn validate_positive(&mut self, field: &str, value: f32) {
        if value <= 0.0 {
            self.add_error(ConfigError::NegativeValue {
                field: field.to_string(),
                value,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_builder_no_errors() {
        let builder = ValidationBuilder::new();
        assert!(builder.finish().is_ok());
    }

    #[test]
    fn test_validation_builder_single_error() {
        let mut builder = ValidationBuilder::new();
        builder.validate_range("test_field", 1.5, 0.0, 1.0);
        let result = builder.finish();
        assert!(matches!(result, Err(ConfigError::OutOfRange { .. })));
    }

    #[test]
    fn test_validation_builder_multiple_errors() {
        let mut builder = ValidationBuilder::new();
        builder.validate_range("field1", 1.5, 0.0, 1.0);
        builder.validate_non_negative("field2", -0.5);
        let result = builder.finish();
        assert!(matches!(result, Err(ConfigError::Multiple(_))));
    }

    #[test]
    fn test_error_display() {
        let error = ConfigError::OutOfRange {
            field: "test.field".to_string(),
            value: 1.5,
            min: 0.0,
            max: 1.0,
        };
        let display = format!("{}", error);
        assert!(display.contains("test.field"));
        assert!(display.contains("1.5"));
    }
}
