use crate::input::CalculationInput;
use crate::metadata::*;
use crate::output::*;
use crate::traits::Calculation;
use crate::types::PriceField;

/// Simple Moving Average calculation.
pub struct Sma;

impl Sma {
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

impl Calculation for Sma {
    fn metadata(&self) -> CalculationMetadata {
        CalculationMetadata {
            id: "sma".into(),
            title: "Simple Moving Average".into(),
            description: "Arithmetic mean of closing prices over the specified period.".into(),
            parameters: vec![ParameterDef {
                name: "period".into(),
                description: "Number of candles to average".into(),
                param_type: ParameterType::Integer,
                default_value: Some(ParamValue::Integer(20)),
                required: false,
            }],
            price_fields: vec![PriceField::Close],
            min_candles_description: "Equal to the period parameter".into(),
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

        let mut sum: f64 = closes[..period].iter().sum();
        values[period - 1] = sum / period as f64;

        for i in period..closes.len() {
            sum += closes[i] - closes[i - period];
            values[i] = sum / period as f64;
        }

        Ok(CalculationOutput { values })
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
    fn sma_5_known_values() {
        let input = CalculationInput {
            candles: make_candles(&[10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0]),
            params: HashMap::from([("period".into(), ParamValue::Integer(5))]),
        };
        let result = Sma.calculate(&input).unwrap();
        assert!(result.values[0..4].iter().all(|v| v.is_nan()));
        assert!((result.values[4] - 12.0).abs() < 1e-10);
        assert!((result.values[5] - 13.0).abs() < 1e-10);
        assert!((result.values[6] - 14.0).abs() < 1e-10);
    }

    #[test]
    fn sma_default_period() {
        let input = CalculationInput {
            candles: make_candles(&vec![100.0; 25]),
            params: HashMap::new(),
        };
        let result = Sma.calculate(&input).unwrap();
        // Default period is 20, so first 19 are NaN
        assert!(result.values[18].is_nan());
        assert!((result.values[19] - 100.0).abs() < 1e-10);
    }

    #[test]
    fn sma_exact_min_candles() {
        let input = CalculationInput {
            candles: make_candles(&[1.0, 2.0, 3.0, 4.0, 5.0]),
            params: HashMap::from([("period".into(), ParamValue::Integer(5))]),
        };
        let result = Sma.calculate(&input).unwrap();
        assert!(result.values[0..4].iter().all(|v| v.is_nan()));
        assert!((result.values[4] - 3.0).abs() < 1e-10);
    }

    #[test]
    fn sma_insufficient_data() {
        let input = CalculationInput {
            candles: make_candles(&[10.0, 11.0, 12.0]),
            params: HashMap::from([("period".into(), ParamValue::Integer(5))]),
        };
        let err = Sma.calculate(&input).unwrap_err();
        assert_eq!(
            err,
            CalculationError::InsufficientData {
                required: 5,
                provided: 3
            }
        );
    }

    #[test]
    fn sma_invalid_period_negative() {
        let input = CalculationInput {
            candles: make_candles(&vec![10.0; 50]),
            params: HashMap::from([("period".into(), ParamValue::Integer(-5))]),
        };
        assert!(matches!(
            Sma.calculate(&input),
            Err(CalculationError::InvalidParameter { .. })
        ));
    }

    #[test]
    fn sma_invalid_period_zero() {
        let input = CalculationInput {
            candles: make_candles(&vec![10.0; 50]),
            params: HashMap::from([("period".into(), ParamValue::Integer(0))]),
        };
        assert!(matches!(
            Sma.calculate(&input),
            Err(CalculationError::InvalidParameter { .. })
        ));
    }

    #[test]
    fn sma_invalid_period_wrong_type() {
        let input = CalculationInput {
            candles: make_candles(&vec![10.0; 50]),
            params: HashMap::from([("period".into(), ParamValue::Float(5.5))]),
        };
        assert!(matches!(
            Sma.calculate(&input),
            Err(CalculationError::InvalidParameter { .. })
        ));
    }

    #[test]
    fn sma_constant_prices() {
        let input = CalculationInput {
            candles: make_candles(&vec![42.0; 10]),
            params: HashMap::from([("period".into(), ParamValue::Integer(5))]),
        };
        let result = Sma.calculate(&input).unwrap();
        for v in &result.values[4..] {
            assert!((v - 42.0).abs() < 1e-10);
        }
    }

    #[test]
    fn sma_metadata_is_correct() {
        let meta = Sma.metadata();
        assert_eq!(meta.id, "sma");
        assert_eq!(meta.title, "Simple Moving Average");
        assert!(!meta.description.is_empty());
        assert_eq!(meta.parameters.len(), 1);
        assert_eq!(meta.parameters[0].name, "period");
        assert_eq!(meta.price_fields, vec![PriceField::Close]);
    }
}
