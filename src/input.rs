use std::collections::HashMap;

use crate::metadata::ParamValue;
use crate::output::CalculationOutput;
use crate::types::Candle;

/// Uniform input contract for every calculation.
#[derive(Debug, Clone)]
pub struct CalculationInput {
    /// The candle data, ordered oldest-first.
    pub candles: Vec<Candle>,
    /// Named parameters (e.g., "period" -> 14).
    pub params: HashMap<String, ParamValue>,
    /// Pre-computed outputs from dependency calculations, keyed by calculation id.
    /// Empty for standalone calculations.
    pub dependencies: HashMap<String, CalculationOutput>,
}

impl CalculationInput {
    /// Create an input with no dependencies (the common case).
    pub fn new(candles: Vec<Candle>, params: HashMap<String, ParamValue>) -> Self {
        Self {
            candles,
            params,
            dependencies: HashMap::new(),
        }
    }

    /// Create an input with pre-computed dependency outputs.
    pub fn with_dependencies(
        candles: Vec<Candle>,
        params: HashMap<String, ParamValue>,
        dependencies: HashMap<String, CalculationOutput>,
    ) -> Self {
        Self {
            candles,
            params,
            dependencies,
        }
    }
}
