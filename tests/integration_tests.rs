use market_calculations::*;
use std::collections::HashMap;
use std::sync::Arc;

fn make_candles(closes: &[f64]) -> Vec<Candle> {
    closes
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
        .collect()
}

fn make_intraday_candles() -> Vec<Candle> {
    let ms_per_day = 24 * 60 * 60 * 1000_i64;
    let mut candles = Vec::new();

    // 3 trading days, each with 4 candles
    for day in 0..3 {
        for i in 0..4 {
            candles.push(Candle {
                timestamp: ms_per_day * day + i * 60_000,
                open: 100.0 + day as f64 * 2.0 + i as f64,
                high: 102.0 + day as f64 * 2.0 + i as f64,
                low: 98.0 + day as f64 * 2.0,
                close: 101.0 + day as f64 * 2.0 + i as f64,
                volume: 1000.0,
            });
        }
    }
    candles
}

#[test]
fn registry_with_defaults_has_all_builtins() {
    let registry = CalculationRegistry::with_defaults();
    assert!(registry.get("sma").is_some());
    assert!(registry.get("ema").is_some());
    assert!(registry.get("rsi").is_some());
    assert!(registry.get("daily_profile").is_some());
    assert!(registry.get("range_trend").is_some());
    assert_eq!(registry.list_ids().len(), 5);
}

#[test]
fn call_sma_through_registry() {
    let registry = CalculationRegistry::with_defaults();
    let sma = registry.get("sma").unwrap();
    let input = CalculationInput::new(
        make_candles(&[10.0, 11.0, 12.0, 13.0, 14.0]),
        HashMap::from([("period".into(), ParamValue::Integer(3))]),
    );
    let result = sma.calculate(&input).unwrap();
    let values = result.values().unwrap();
    assert!((values[2] - 11.0).abs() < 1e-10);
    assert!((values[3] - 12.0).abs() < 1e-10);
    assert!((values[4] - 13.0).abs() < 1e-10);
}

#[test]
fn call_ema_through_registry() {
    let registry = CalculationRegistry::with_defaults();
    let ema = registry.get("ema").unwrap();
    let input = CalculationInput::new(
        make_candles(&[10.0, 11.0, 12.0, 13.0, 14.0]),
        HashMap::from([("period".into(), ParamValue::Integer(3))]),
    );
    let result = ema.calculate(&input).unwrap();
    assert!(!result.values().unwrap()[2].is_nan());
}

#[test]
fn call_rsi_through_registry() {
    let registry = CalculationRegistry::with_defaults();
    let rsi = registry.get("rsi").unwrap();
    let closes: Vec<f64> = (0..20).map(|i| 100.0 + (i as f64) * 0.5).collect();
    let input = CalculationInput::new(
        make_candles(&closes),
        HashMap::from([("period".into(), ParamValue::Integer(14))]),
    );
    let result = rsi.calculate(&input).unwrap();
    let values = result.values().unwrap();
    assert!(!values[14].is_nan());
    assert!(values[14] >= 0.0 && values[14] <= 100.0);
}

#[test]
fn manifest_round_trips_through_json() {
    let registry = CalculationRegistry::with_defaults();
    let json = registry.export_manifest().unwrap();
    let parsed: Vec<CalculationMetadata> = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.len(), 5);

    for meta in &parsed {
        assert!(!meta.id.is_empty());
        assert!(!meta.title.is_empty());
        assert!(!meta.description.is_empty());
        assert!(!meta.min_candles_description.is_empty());
    }

    // Verify new metadata fields are present
    let range_trend_meta = parsed.iter().find(|m| m.id == "range_trend").unwrap();
    assert_eq!(range_trend_meta.output_series.len(), 2);
    assert_eq!(range_trend_meta.dependencies.len(), 1);

    let daily_meta = parsed.iter().find(|m| m.id == "daily_profile").unwrap();
    assert_eq!(daily_meta.output_series.len(), 5);
    assert!(daily_meta.dependencies.is_empty());
}

#[test]
fn custom_calculation_can_be_registered() {
    use market_calculations::metadata::{ParameterDef, ParameterType};
    use market_calculations::types::PriceField;

    struct CustomCalc;

    impl Calculation for CustomCalc {
        fn metadata(&self) -> CalculationMetadata {
            CalculationMetadata {
                id: "custom_test".into(),
                title: "Custom Test Calculation".into(),
                description: "A test custom calculation".into(),
                parameters: vec![ParameterDef {
                    name: "factor".into(),
                    description: "Multiplication factor".into(),
                    param_type: ParameterType::Float,
                    default_value: Some(ParamValue::Float(1.0)),
                    required: false,
                }],
                price_fields: vec![PriceField::Close],
                min_candles_description: "1 candle minimum".into(),
                output_series: vec![OutputSeriesDef {
                    name: "value".into(),
                    description: "Doubled close prices".into(),
                    series_type: SeriesType::Numeric,
                }],
                dependencies: vec![],
            }
        }

        fn min_candles(&self, _input: &CalculationInput) -> usize {
            1
        }

        fn calculate(
            &self,
            input: &CalculationInput,
        ) -> Result<CalculationOutput, CalculationError> {
            self.validate_input(input)?;
            let values: Vec<f64> = input.candles.iter().map(|c| c.close * 2.0).collect();
            Ok(CalculationOutput::single(values))
        }
    }

    let mut registry = CalculationRegistry::with_defaults();
    registry.register(Arc::new(CustomCalc));

    assert_eq!(registry.list_ids().len(), 6);
    assert!(registry.get("custom_test").is_some());

    let input = CalculationInput::new(make_candles(&[10.0, 20.0]), HashMap::new());
    let result = registry
        .get("custom_test")
        .unwrap()
        .calculate(&input)
        .unwrap();
    let values = result.values().unwrap();
    assert!((values[0] - 20.0).abs() < 1e-10);
    assert!((values[1] - 40.0).abs() < 1e-10);

    let json = registry.export_manifest().unwrap();
    assert!(json.contains("custom_test"));
}

#[test]
fn single_series_calcs_uniform_contract() {
    let registry = CalculationRegistry::with_defaults();
    let closes: Vec<f64> = (0..30).map(|i| 100.0 + (i as f64) * 0.5).collect();

    // All single-series calcs (sma, ema, rsi) can be called uniformly
    for id in ["sma", "ema", "rsi"] {
        let calc = registry.get(id).unwrap();
        let input = CalculationInput::new(
            make_candles(&closes),
            HashMap::from([("period".into(), ParamValue::Integer(5))]),
        );
        let result = calc.calculate(&input).unwrap();
        assert_eq!(
            result.values().unwrap().len(),
            closes.len(),
            "{} output length mismatch",
            id
        );
    }
}

#[test]
fn pipeline_runs_derivative_calculation() {
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

#[test]
fn pipeline_runs_standalone_calc() {
    let registry = CalculationRegistry::with_defaults();
    let pipeline = Pipeline::new(&registry);
    let closes: Vec<f64> = (0..30).map(|i| 100.0 + i as f64).collect();

    let params = HashMap::from([("period".into(), ParamValue::Integer(5))]);
    let result = pipeline
        .run("sma", &make_candles(&closes), &params)
        .unwrap();
    assert!(result.values().is_some());
    assert_eq!(result.values().unwrap().len(), 30);
}
