use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::types::PriceField;

/// Describes a single parameter that a calculation accepts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterDef {
    pub name: String,
    pub description: String,
    pub param_type: ParameterType,
    pub default_value: Option<ParamValue>,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParameterType {
    Integer,
    Float,
    String,
    Boolean,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ParamValue {
    Integer(i64),
    Float(f64),
    String(String),
    Boolean(bool),
}

/// Describes a single output series produced by a calculation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputSeriesDef {
    /// Key name in the output HashMap, e.g. "daily_range_pct".
    pub name: String,
    /// Human-readable description.
    pub description: String,
    /// What the values represent.
    pub series_type: SeriesType,
}

/// Classification of output series values.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SeriesType {
    /// Continuous numeric (price, index value, etc.)
    Numeric,
    /// Boolean encoded as 1.0 / 0.0.
    Boolean,
    /// Percentage value.
    Percentage,
}

/// Declares that a calculation depends on another calculation's output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyDef {
    /// The id of the required calculation (must exist in registry).
    pub calculation_id: String,
    /// Which output series from that calculation are needed. Empty means all.
    pub required_series: Vec<String>,
    /// Default params to use when running the dependency.
    #[serde(default)]
    pub default_params: HashMap<String, ParamValue>,
}

/// Full metadata for a registered calculation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalculationMetadata {
    /// Unique identifier, e.g. "rsi", "sma", "ema".
    pub id: String,
    /// Human-readable title, e.g. "Relative Strength Index".
    pub title: String,
    /// Longer description of what this calculation does.
    pub description: String,
    /// Which parameters the calculation accepts.
    pub parameters: Vec<ParameterDef>,
    /// Which price fields the calculation reads.
    pub price_fields: Vec<PriceField>,
    /// Description of the minimum candle requirement formula.
    pub min_candles_description: String,
    /// Describes the output series this calculation produces.
    #[serde(default)]
    pub output_series: Vec<OutputSeriesDef>,
    /// Other calculations whose output this calculation requires.
    #[serde(default)]
    pub dependencies: Vec<DependencyDef>,
}
