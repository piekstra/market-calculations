use std::collections::HashMap;

use crate::metadata::ParamValue;
use crate::types::Candle;

/// Uniform input contract for every calculation.
#[derive(Debug, Clone)]
pub struct CalculationInput {
    /// The candle data, ordered oldest-first.
    pub candles: Vec<Candle>,
    /// Named parameters (e.g., "period" -> 14).
    pub params: HashMap<String, ParamValue>,
}
