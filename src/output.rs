use serde::{Deserialize, Serialize};
use std::fmt;

/// The result of a calculation: one f64 value per input candle
/// where the calculation is defined. Leading entries are `f64::NAN`
/// for the warm-up period.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalculationOutput {
    /// One value per candle. Values during the warm-up period
    /// where the calculation cannot yet produce a result are `f64::NAN`.
    pub values: Vec<f64>,
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
