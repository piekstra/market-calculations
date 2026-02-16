use crate::input::CalculationInput;
use crate::metadata::*;
use crate::output::*;
use crate::traits::Calculation;
use crate::types::PriceField;

/// Relative Strength Index calculation using Wilder's smoothing method.
pub struct Rsi;

impl Rsi {
    fn get_period(input: &CalculationInput) -> Result<usize, CalculationError> {
        match input.params.get("period") {
            Some(ParamValue::Integer(p)) if *p > 0 => Ok(*p as usize),
            Some(ParamValue::Integer(p)) => Err(CalculationError::InvalidParameter {
                name: "period".into(),
                reason: format!("period must be positive, got {}", p),
            }),
            Some(_) => Err(CalculationError::InvalidParameter {
                name: "period".into(),
                reason: "period must be an integer".into(),
            }),
            None => Ok(14),
        }
    }
}

impl Calculation for Rsi {
    fn metadata(&self) -> CalculationMetadata {
        CalculationMetadata {
            id: "rsi".into(),
            title: "Relative Strength Index".into(),
            description: "Momentum oscillator measuring speed and magnitude of price changes. \
                Uses Wilder's smoothing method. Output range: 0-100."
                .into(),
            parameters: vec![ParameterDef {
                name: "period".into(),
                description: "Lookback period for RSI calculation".into(),
                param_type: ParameterType::Integer,
                default_value: Some(ParamValue::Integer(14)),
                required: false,
            }],
            price_fields: vec![PriceField::Close],
            min_candles_description: "period + 1 (need `period` price changes)".into(),
            output_series: vec![OutputSeriesDef {
                name: "value".into(),
                description: "RSI values (0-100)".into(),
                series_type: SeriesType::Numeric,
            }],
            dependencies: vec![],
        }
    }

    fn min_candles(&self, input: &CalculationInput) -> usize {
        Self::get_period(input).unwrap_or(14) + 1
    }

    fn calculate(&self, input: &CalculationInput) -> Result<CalculationOutput, CalculationError> {
        self.validate_input(input)?;
        let period = Self::get_period(input)?;
        let closes: Vec<f64> = input.candles.iter().map(|c| c.close).collect();
        let n = closes.len();
        let mut values = vec![f64::NAN; n];

        // Compute price changes
        let mut gains = vec![0.0f64; n];
        let mut losses = vec![0.0f64; n];
        for i in 1..n {
            let change = closes[i] - closes[i - 1];
            if change > 0.0 {
                gains[i] = change;
            } else {
                losses[i] = -change;
            }
        }

        // Initial average gain/loss over first `period` changes
        let mut avg_gain: f64 = gains[1..=period].iter().sum::<f64>() / period as f64;
        let mut avg_loss: f64 = losses[1..=period].iter().sum::<f64>() / period as f64;

        // First RSI value at index `period`
        if avg_loss == 0.0 {
            values[period] = 100.0;
        } else {
            let rs = avg_gain / avg_loss;
            values[period] = 100.0 - (100.0 / (1.0 + rs));
        }

        // Wilder's smoothing for subsequent values
        for i in (period + 1)..n {
            avg_gain = (avg_gain * (period as f64 - 1.0) + gains[i]) / period as f64;
            avg_loss = (avg_loss * (period as f64 - 1.0) + losses[i]) / period as f64;
            if avg_loss == 0.0 {
                values[i] = 100.0;
            } else {
                let rs = avg_gain / avg_loss;
                values[i] = 100.0 - (100.0 / (1.0 + rs));
            }
        }

        Ok(CalculationOutput::single(values))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Candle;
    use std::collections::HashMap;

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
    fn rsi_warmup_is_nan() {
        let closes: Vec<f64> = (0..30).map(|i| 100.0 + (i as f64) * 0.5).collect();
        let input = CalculationInput::new(
            make_candles(&closes),
            HashMap::from([("period".into(), ParamValue::Integer(14))]),
        );
        let result = Rsi.calculate(&input).unwrap();
        let values = result.values().unwrap();
        assert!(values[0..14].iter().all(|v| v.is_nan()));
        assert!(!values[14].is_nan());
    }

    #[test]
    fn rsi_monotonically_increasing_near_100() {
        let closes: Vec<f64> = (0..20).map(|i| 100.0 + i as f64).collect();
        let input = CalculationInput::new(
            make_candles(&closes),
            HashMap::from([("period".into(), ParamValue::Integer(14))]),
        );
        let result = Rsi.calculate(&input).unwrap();
        let values = result.values().unwrap();
        for v in &values[14..] {
            assert!((v - 100.0).abs() < 1e-10);
        }
    }

    #[test]
    fn rsi_monotonically_decreasing_near_0() {
        let closes: Vec<f64> = (0..20).map(|i| 200.0 - i as f64).collect();
        let input = CalculationInput::new(
            make_candles(&closes),
            HashMap::from([("period".into(), ParamValue::Integer(14))]),
        );
        let result = Rsi.calculate(&input).unwrap();
        let values = result.values().unwrap();
        for v in &values[14..] {
            assert!(v.abs() < 1e-10);
        }
    }

    #[test]
    fn rsi_output_in_range_0_100() {
        let closes = vec![
            44.0, 44.34, 44.09, 43.61, 44.33, 44.83, 45.10, 45.42, 45.84, 46.08, 45.89, 46.03,
            45.61, 46.28, 46.28, 46.00, 46.03, 46.41, 46.22, 45.64,
        ];
        let input = CalculationInput::new(
            make_candles(&closes),
            HashMap::from([("period".into(), ParamValue::Integer(14))]),
        );
        let result = Rsi.calculate(&input).unwrap();
        let values = result.values().unwrap();
        for v in &values[14..] {
            assert!(*v >= 0.0 && *v <= 100.0, "RSI {} out of range", v);
        }
    }

    #[test]
    fn rsi_exact_min_candles() {
        let input = CalculationInput::new(
            make_candles(&vec![100.0; 15]),
            HashMap::from([("period".into(), ParamValue::Integer(14))]),
        );
        let result = Rsi.calculate(&input).unwrap();
        let values = result.values().unwrap();
        assert_eq!(values.len(), 15);
        assert!((values[14] - 100.0).abs() < 1e-10);
    }

    #[test]
    fn rsi_insufficient_data() {
        let input = CalculationInput::new(
            make_candles(&vec![100.0; 14]),
            HashMap::from([("period".into(), ParamValue::Integer(14))]),
        );
        assert_eq!(
            Rsi.calculate(&input).unwrap_err(),
            CalculationError::InsufficientData {
                required: 15,
                provided: 14
            }
        );
    }

    #[test]
    fn rsi_small_period() {
        let input = CalculationInput::new(
            make_candles(&[10.0, 12.0, 11.0, 13.0]),
            HashMap::from([("period".into(), ParamValue::Integer(3))]),
        );
        let result = Rsi.calculate(&input).unwrap();
        let values = result.values().unwrap();
        assert!(values[0..3].iter().all(|v| v.is_nan()));
        assert!(!values[3].is_nan());
        assert!(values[3] >= 0.0 && values[3] <= 100.0);
    }

    #[test]
    fn rsi_invalid_period() {
        let input = CalculationInput::new(
            make_candles(&vec![10.0; 50]),
            HashMap::from([("period".into(), ParamValue::Integer(-1))]),
        );
        assert!(matches!(
            Rsi.calculate(&input),
            Err(CalculationError::InvalidParameter { .. })
        ));
    }

    #[test]
    fn rsi_metadata_is_correct() {
        let meta = Rsi.metadata();
        assert_eq!(meta.id, "rsi");
        assert_eq!(meta.title, "Relative Strength Index");
        assert!(meta.description.contains("Wilder"));
        assert_eq!(meta.parameters.len(), 1);
        assert_eq!(meta.parameters[0].name, "period");
        assert_eq!(meta.price_fields, vec![PriceField::Close]);
        assert_eq!(meta.output_series.len(), 1);
        assert!(meta.dependencies.is_empty());
    }
}
