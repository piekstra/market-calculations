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
}
