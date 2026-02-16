use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

/// Well-known key for single-series calculations.
pub const PRIMARY_SERIES: &str = "value";

/// The result of a calculation: one or more named series of f64 values.
/// Single-output calculations use the key "value" via `CalculationOutput::single()`.
/// Multi-output calculations provide multiple named series via `CalculationOutput::multi()`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalculationOutput {
    /// Named output series. Single-output calcs use the key "value".
    pub series: HashMap<String, Vec<f64>>,
}

impl CalculationOutput {
    /// Convenience constructor for single-series calculations.
    pub fn single(values: Vec<f64>) -> Self {
        let mut series = HashMap::new();
        series.insert(PRIMARY_SERIES.to_string(), values);
        Self { series }
    }

    /// Convenience constructor for multi-series calculations.
    pub fn multi(series: HashMap<String, Vec<f64>>) -> Self {
        Self { series }
    }

    /// Get the primary series (shorthand for single-output calcs).
    pub fn values(&self) -> Option<&Vec<f64>> {
        self.series.get(PRIMARY_SERIES)
    }

    /// Get a named series.
    pub fn get(&self, name: &str) -> Option<&Vec<f64>> {
        self.series.get(name)
    }
}

/// Errors that a calculation can produce.
#[derive(Debug, Clone, PartialEq)]
pub enum CalculationError {
    /// Not enough candles for the requested parameters.
    InsufficientData { required: usize, provided: usize },
    /// A required parameter is missing.
    MissingParameter(String),
    /// A parameter has the wrong type or is out of range.
    InvalidParameter { name: String, reason: String },
    /// Catch-all for calculation-specific errors.
    CalculationFailed(String),
}

impl fmt::Display for CalculationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InsufficientData { required, provided } => {
                write!(f, "Need at least {} candles, got {}", required, provided)
            }
            Self::MissingParameter(name) => write!(f, "Missing required parameter: {}", name),
            Self::InvalidParameter { name, reason } => {
                write!(f, "Invalid parameter '{}': {}", name, reason)
            }
            Self::CalculationFailed(msg) => write!(f, "Calculation failed: {}", msg),
        }
    }
}

impl std::error::Error for CalculationError {}
