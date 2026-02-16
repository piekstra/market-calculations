use crate::input::CalculationInput;
use crate::metadata::*;
use crate::output::*;
use crate::traits::Calculation;
use crate::types::PriceField;

/// Exponential Moving Average calculation.
pub struct Ema;

impl Ema {
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
            None => Ok(20),
        }
    }
}

impl Calculation for Ema {
    fn metadata(&self) -> CalculationMetadata {
        CalculationMetadata {
            id: "ema".into(),
            title: "Exponential Moving Average".into(),
            description: "Weighted moving average giving more weight to recent prices. \
                Uses SMA of the first `period` candles as the seed value."
                .into(),
            parameters: vec![ParameterDef {
                name: "period".into(),
                description: "Smoothing period".into(),
                param_type: ParameterType::Integer,
                default_value: Some(ParamValue::Integer(20)),
                required: false,
            }],
            price_fields: vec![PriceField::Close],
            min_candles_description: "Equal to the period parameter".into(),
            output_series: vec![OutputSeriesDef {
                name: "value".into(),
                description: "EMA values".into(),
                series_type: SeriesType::Numeric,
            }],
            dependencies: vec![],
        }
    }

    fn min_candles(&self, input: &CalculationInput) -> usize {
        Self::get_period(input).unwrap_or(20)
    }

    fn calculate(&self, input: &CalculationInput) -> Result<CalculationOutput, CalculationError> {
        self.validate_input(input)?;
        let period = Self::get_period(input)?;
        let closes: Vec<f64> = input.candles.iter().map(|c| c.close).collect();
        let mut values = vec![f64::NAN; closes.len()];

        // Seed EMA with SMA of first `period` candles
        let sma_seed: f64 = closes[..period].iter().sum::<f64>() / period as f64;
        values[period - 1] = sma_seed;

        let multiplier = 2.0 / (period as f64 + 1.0);

        for i in period..closes.len() {
            values[i] = (closes[i] - values[i - 1]) * multiplier + values[i - 1];
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
    fn ema_seed_equals_sma() {
        let input = CalculationInput::new(
            make_candles(&[10.0, 11.0, 12.0, 13.0, 14.0]),
            HashMap::from([("period".into(), ParamValue::Integer(5))]),
        );
        let result = Ema.calculate(&input).unwrap();
        assert!((result.values().unwrap()[4] - 12.0).abs() < 1e-10);
    }

    #[test]
    fn ema_subsequent_values() {
        let input = CalculationInput::new(
            make_candles(&[10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0]),
            HashMap::from([("period".into(), ParamValue::Integer(5))]),
        );
        let result = Ema.calculate(&input).unwrap();
        let values = result.values().unwrap();
        let multiplier = 2.0 / 6.0;
        let expected_5 = (15.0 - 12.0) * multiplier + 12.0;
        assert!((values[5] - expected_5).abs() < 1e-10);
        let expected_6 = (16.0 - expected_5) * multiplier + expected_5;
        assert!((values[6] - expected_6).abs() < 1e-10);
    }

    #[test]
    fn ema_warmup_is_nan() {
        let input = CalculationInput::new(
            make_candles(&[10.0, 11.0, 12.0, 13.0, 14.0, 15.0]),
            HashMap::from([("period".into(), ParamValue::Integer(5))]),
        );
        let result = Ema.calculate(&input).unwrap();
        let values = result.values().unwrap();
        assert!(values[0..4].iter().all(|v| v.is_nan()));
        assert!(!values[4].is_nan());
    }

    #[test]
    fn ema_insufficient_data() {
        let input = CalculationInput::new(
            make_candles(&[10.0, 11.0]),
            HashMap::from([("period".into(), ParamValue::Integer(5))]),
        );
        assert_eq!(
            Ema.calculate(&input).unwrap_err(),
            CalculationError::InsufficientData {
                required: 5,
                provided: 2
            }
        );
    }

    #[test]
    fn ema_constant_prices() {
        let input = CalculationInput::new(
            make_candles(&vec![42.0; 10]),
            HashMap::from([("period".into(), ParamValue::Integer(5))]),
        );
        let result = Ema.calculate(&input).unwrap();
        let values = result.values().unwrap();
        for v in &values[4..] {
            assert!((v - 42.0).abs() < 1e-10);
        }
    }

    #[test]
    fn ema_invalid_period() {
        let input = CalculationInput::new(
            make_candles(&vec![10.0; 50]),
            HashMap::from([("period".into(), ParamValue::Integer(-3))]),
        );
        assert!(matches!(
            Ema.calculate(&input),
            Err(CalculationError::InvalidParameter { .. })
        ));
    }

    #[test]
    fn ema_metadata_is_correct() {
        let meta = Ema.metadata();
        assert_eq!(meta.id, "ema");
        assert_eq!(meta.title, "Exponential Moving Average");
        assert!(!meta.description.is_empty());
        assert_eq!(meta.parameters.len(), 1);
        assert_eq!(meta.price_fields, vec![PriceField::Close]);
        assert_eq!(meta.output_series.len(), 1);
        assert!(meta.dependencies.is_empty());
    }
}
