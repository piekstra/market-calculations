use std::collections::HashMap;

use crate::input::CalculationInput;
use crate::metadata::ParamValue;
use crate::output::{CalculationError, CalculationOutput};
use crate::registry::CalculationRegistry;
use crate::types::Candle;

/// Executes a calculation and all its dependencies in the correct order.
pub struct Pipeline<'a> {
    registry: &'a CalculationRegistry,
}

impl<'a> Pipeline<'a> {
    pub fn new(registry: &'a CalculationRegistry) -> Self {
        Self { registry }
    }

    /// Run a calculation by ID, automatically resolving and executing dependencies.
    /// Returns only the target calculation's output.
    pub fn run(
        &self,
        id: &str,
        candles: &[Candle],
        params: &HashMap<String, ParamValue>,
    ) -> Result<CalculationOutput, CalculationError> {
        let mut all = self.run_all(id, candles, params)?;
        all.remove(id).ok_or_else(|| {
            CalculationError::CalculationFailed("Pipeline produced no output".into())
        })
    }

    /// Run a calculation and return ALL intermediate outputs (including dependencies).
    pub fn run_all(
        &self,
        id: &str,
        candles: &[Candle],
        params: &HashMap<String, ParamValue>,
    ) -> Result<HashMap<String, CalculationOutput>, CalculationError> {
        let execution_order = self.registry.resolve_dependencies(id)?;
        let mut outputs: HashMap<String, CalculationOutput> = HashMap::new();

        for calc_id in &execution_order {
            let calc = self.registry.get(calc_id).ok_or_else(|| {
                CalculationError::CalculationFailed(format!("Unknown calculation: '{}'", calc_id))
            })?;
            let meta = calc.metadata();

            // Build dependencies map for this calculation
            let mut deps = HashMap::new();
            for dep_def in &meta.dependencies {
                if let Some(dep_output) = outputs.get(&dep_def.calculation_id) {
                    deps.insert(dep_def.calculation_id.clone(), dep_output.clone());
                }
            }

            // Use caller's params for the target calc; use dependency defaults for others
            let calc_params = if calc_id == id {
                params.clone()
            } else {
                // Find the parent calc's DependencyDef that references this calc
                let parent_meta = self
                    .registry
                    .get(id)
                    .map(|c| c.metadata())
                    .unwrap_or(meta.clone());
                parent_meta
                    .dependencies
                    .iter()
                    .find(|d| d.calculation_id == *calc_id)
                    .map(|d| d.default_params.clone())
                    .unwrap_or_default()
            };

            let input = CalculationInput::with_dependencies(candles.to_vec(), calc_params, deps);

            let output = calc.calculate(&input)?;
            outputs.insert(calc_id.clone(), output);
        }

        Ok(outputs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_intraday_candles() -> Vec<Candle> {
        // 3 trading days, each with 4 candles (simulating 4 intervals per day)
        let ms_per_day = 24 * 60 * 60 * 1000_i64;
        let mut candles = Vec::new();

        // Day 1: uptrend day (high before low = no, high comes after)
        for i in 0..4 {
            candles.push(Candle {
                timestamp: ms_per_day * 0 + i * 60_000,
                open: 100.0 + i as f64,
                high: 101.0 + i as f64,
                low: 99.0 + i as f64,
                close: 100.5 + i as f64,
                volume: 1000.0,
            });
        }

        // Day 2: downtrend day
        for i in 0..4 {
            candles.push(Candle {
                timestamp: ms_per_day * 1 + i * 60_000,
                open: 105.0 - i as f64,
                high: 106.0 - i as f64,
                low: 104.0 - i as f64,
                close: 104.5 - i as f64,
                volume: 1000.0,
            });
        }

        // Day 3: mixed day
        for i in 0..4 {
            let offset = if i % 2 == 0 { 1.0 } else { -1.0 };
            candles.push(Candle {
                timestamp: ms_per_day * 2 + i * 60_000,
                open: 102.0 + offset,
                high: 103.0 + offset,
                low: 101.0 + offset,
                close: 102.5 + offset,
                volume: 1000.0,
            });
        }

        candles
    }

    #[test]
    fn pipeline_runs_standalone_calc() {
        let registry = CalculationRegistry::with_defaults();
        let pipeline = Pipeline::new(&registry);

        // SMA has no dependencies
        let closes: Vec<f64> = (0..30).map(|i| 100.0 + i as f64).collect();
        let candles: Vec<Candle> = closes
            .iter()
            .enumerate()
            .map(|(i, &c)| Candle {
                timestamp: (i as i64) * 60_000,
                open: c,
                high: c,
                low: c,
                close: c,
                volume: 1000.0,
            })
            .collect();

        let params = HashMap::from([("period".into(), ParamValue::Integer(5))]);
        let result = pipeline.run("sma", &candles, &params).unwrap();
        assert!(result.values().is_some());
    }

    #[test]
    fn pipeline_runs_daily_profile() {
        let registry = CalculationRegistry::with_defaults();
        let pipeline = Pipeline::new(&registry);
        let candles = make_intraday_candles();

        let result = pipeline
            .run("daily_profile", &candles, &HashMap::new())
            .unwrap();
        assert!(result.get("daily_range_pct").is_some());
        assert!(result.get("high_before_low").is_some());
        assert!(result.get("tightness").is_some());
        assert!(result.get("gap_pct").is_some());
        assert!(result.get("open_to_close_pct").is_some());
        assert_eq!(result.get("daily_range_pct").unwrap().len(), 3);
    }

    #[test]
    fn pipeline_resolves_dependencies_for_range_trend() {
        let registry = CalculationRegistry::with_defaults();
        let pipeline = Pipeline::new(&registry);
        let candles = make_intraday_candles();

        let params = HashMap::from([("window".into(), ParamValue::Integer(2))]);
        let result = pipeline.run("range_trend", &candles, &params).unwrap();
        assert!(result.get("trend_score").is_some());
        assert!(result.get("avg_range").is_some());
    }

    #[test]
    fn pipeline_run_all_includes_intermediates() {
        let registry = CalculationRegistry::with_defaults();
        let pipeline = Pipeline::new(&registry);
        let candles = make_intraday_candles();

        let params = HashMap::from([("window".into(), ParamValue::Integer(2))]);
        let all = pipeline.run_all("range_trend", &candles, &params).unwrap();
        assert!(all.contains_key("daily_profile"));
        assert!(all.contains_key("range_trend"));
    }
}
