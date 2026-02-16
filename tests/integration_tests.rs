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

#[test]
fn registry_with_defaults_has_all_builtins() {
    let registry = CalculationRegistry::with_defaults();
    assert!(registry.get("sma").is_some());
    assert!(registry.get("ema").is_some());
    assert!(registry.get("rsi").is_some());
    assert_eq!(registry.list_ids().len(), 3);
}

#[test]
fn call_sma_through_registry() {
    let registry = CalculationRegistry::with_defaults();
    let sma = registry.get("sma").unwrap();
    let input = CalculationInput {
        candles: make_candles(&[10.0, 11.0, 12.0, 13.0, 14.0]),
        params: HashMap::from([("period".into(), ParamValue::Integer(3))]),
    };
    let result = sma.calculate(&input).unwrap();
    assert!((result.values[2] - 11.0).abs() < 1e-10);
    assert!((result.values[3] - 12.0).abs() < 1e-10);
    assert!((result.values[4] - 13.0).abs() < 1e-10);
}

#[test]
fn call_ema_through_registry() {
    let registry = CalculationRegistry::with_defaults();
    let ema = registry.get("ema").unwrap();
    let input = CalculationInput {
        candles: make_candles(&[10.0, 11.0, 12.0, 13.0, 14.0]),
        params: HashMap::from([("period".into(), ParamValue::Integer(3))]),
    };
    let result = ema.calculate(&input).unwrap();
    assert!(!result.values[2].is_nan());
}

#[test]
fn call_rsi_through_registry() {
    let registry = CalculationRegistry::with_defaults();
    let rsi = registry.get("rsi").unwrap();
    let closes: Vec<f64> = (0..20).map(|i| 100.0 + (i as f64) * 0.5).collect();
    let input = CalculationInput {
        candles: make_candles(&closes),
        params: HashMap::from([("period".into(), ParamValue::Integer(14))]),
    };
    let result = rsi.calculate(&input).unwrap();
    assert!(!result.values[14].is_nan());
    assert!(result.values[14] >= 0.0 && result.values[14] <= 100.0);
}

#[test]
fn manifest_round_trips_through_json() {
    let registry = CalculationRegistry::with_defaults();
    let json = registry.export_manifest().unwrap();
    let parsed: Vec<CalculationMetadata> = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.len(), 3);

    for meta in &parsed {
        assert!(!meta.id.is_empty());
        assert!(!meta.title.is_empty());
        assert!(!meta.description.is_empty());
        assert!(!meta.min_candles_description.is_empty());
    }
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
            Ok(CalculationOutput { values })
        }
    }

    let mut registry = CalculationRegistry::with_defaults();
    registry.register(Arc::new(CustomCalc));

    assert_eq!(registry.list_ids().len(), 4);
    assert!(registry.get("custom_test").is_some());

    let input = CalculationInput {
        candles: make_candles(&[10.0, 20.0]),
        params: HashMap::new(),
    };
    let result = registry
        .get("custom_test")
        .unwrap()
        .calculate(&input)
        .unwrap();
    assert!((result.values[0] - 20.0).abs() < 1e-10);
    assert!((result.values[1] - 40.0).abs() < 1e-10);

    // Custom calc appears in manifest
    let json = registry.export_manifest().unwrap();
    assert!(json.contains("custom_test"));
}

#[test]
fn uniform_contract_any_calculation_same_interface() {
    let registry = CalculationRegistry::with_defaults();
    let closes: Vec<f64> = (0..30).map(|i| 100.0 + (i as f64) * 0.5).collect();

    // Every registered calculation can be called with the same input shape
    for id in registry.list_ids() {
        let calc = registry.get(&id).unwrap();
        let meta = calc.metadata();

        let input = CalculationInput {
            candles: make_candles(&closes),
            params: HashMap::from([("period".into(), ParamValue::Integer(5))]),
        };

        let result = calc.calculate(&input).unwrap();
        assert_eq!(
            result.values.len(),
            closes.len(),
            "{} output length mismatch",
            meta.id
        );
    }
}
