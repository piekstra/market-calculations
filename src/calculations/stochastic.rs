use std::collections::HashMap;

use crate::input::CalculationInput;
use crate::metadata::*;
use crate::output::*;
use crate::traits::Calculation;
use crate::types::PriceField;

/// Stochastic Oscillator: momentum indicator comparing closing price to price range.
pub struct Stochastic;

impl Stochastic {
    fn get_k_period(input: &CalculationInput) -> Result<usize, CalculationError> {
        match input.params.get("k_period") {
            Some(ParamValue::Integer(p)) if *p > 0 => Ok(*p as usize),
            Some(ParamValue::Integer(p)) => Err(CalculationError::InvalidParameter {
                name: "k_period".into(),
                reason: format!("k_period must be positive, got {}", p),
            }),
            Some(_) => Err(CalculationError::InvalidParameter {
                name: "k_period".into(),
                reason: "k_period must be an integer".into(),
            }),
            None => Ok(14),
        }
    }

    fn get_d_period(input: &CalculationInput) -> Result<usize, CalculationError> {
        match input.params.get("d_period") {
            Some(ParamValue::Integer(p)) if *p > 0 => Ok(*p as usize),
            Some(ParamValue::Integer(p)) => Err(CalculationError::InvalidParameter {
                name: "d_period".into(),
                reason: format!("d_period must be positive, got {}", p),
            }),
            Some(_) => Err(CalculationError::InvalidParameter {
                name: "d_period".into(),
                reason: "d_period must be an integer".into(),
            }),
            None => Ok(3),
        }
    }

    fn get_smooth_k(input: &CalculationInput) -> Result<usize, CalculationError> {
        match input.params.get("smooth_k") {
            Some(ParamValue::Integer(p)) if *p > 0 => Ok(*p as usize),
            Some(ParamValue::Integer(p)) => Err(CalculationError::InvalidParameter {
                name: "smooth_k".into(),
                reason: format!("smooth_k must be positive, got {}", p),
            }),
            Some(_) => Err(CalculationError::InvalidParameter {
                name: "smooth_k".into(),
                reason: "smooth_k must be an integer".into(),
            }),
            None => Ok(3),
        }
    }

    /// Simple moving average over a slice, returning NaN for warmup.
    fn sma(data: &[f64], period: usize) -> Vec<f64> {
        let n = data.len();
        let mut result = vec![f64::NAN; n];
        for i in (period - 1)..n {
            let sum: f64 = data[(i + 1 - period)..=i]
                .iter()
                .filter(|v| !v.is_nan())
                .sum();
            let count = data[(i + 1 - period)..=i]
                .iter()
                .filter(|v| !v.is_nan())
                .count();
            if count == period {
                result[i] = sum / period as f64;
            }
        }
        result
    }
}

impl Calculation for Stochastic {
    fn metadata(&self) -> CalculationMetadata {
        CalculationMetadata {
            id: "stochastic".into(),
            title: "Stochastic Oscillator".into(),
            description: "Momentum indicator comparing closing price to price range. \
                %K = (close - lowest_low) / (highest_high - lowest_low) * 100. \
                Smoothed %K uses SMA, %D = SMA of smoothed %K."
                .into(),
            parameters: vec![
                ParameterDef {
                    name: "k_period".into(),
                    description: "Lookback period for raw %K".into(),
                    param_type: ParameterType::Integer,
                    default_value: Some(ParamValue::Integer(14)),
                    required: false,
                },
                ParameterDef {
                    name: "d_period".into(),
                    description: "SMA period for %D".into(),
                    param_type: ParameterType::Integer,
                    default_value: Some(ParamValue::Integer(3)),
                    required: false,
                },
                ParameterDef {
                    name: "smooth_k".into(),
                    description: "SMA smoothing period for %K".into(),
                    param_type: ParameterType::Integer,
                    default_value: Some(ParamValue::Integer(3)),
                    required: false,
                },
            ],
            price_fields: vec![PriceField::High, PriceField::Low, PriceField::Close],
            min_candles_description: "k_period + smooth_k + d_period - 2".into(),
            output_series: vec![
                OutputSeriesDef {
                    name: "k".into(),
                    description: "Smoothed %K line (0-100)".into(),
                    series_type: SeriesType::Numeric,
                },
                OutputSeriesDef {
                    name: "d".into(),
                    description: "%D line (SMA of %K, 0-100)".into(),
                    series_type: SeriesType::Numeric,
                },
            ],
            dependencies: vec![],
        }
    }

    fn min_candles(&self, input: &CalculationInput) -> usize {
        let k = Self::get_k_period(input).unwrap_or(14);
        let d = Self::get_d_period(input).unwrap_or(3);
        let sk = Self::get_smooth_k(input).unwrap_or(3);
        k + sk + d - 2
    }

    fn calculate(&self, input: &CalculationInput) -> Result<CalculationOutput, CalculationError> {
        self.validate_input(input)?;
        let k_period = Self::get_k_period(input)?;
        let d_period = Self::get_d_period(input)?;
        let smooth_k = Self::get_smooth_k(input)?;
        let candles = &input.candles;
        let n = candles.len();

        // Step 1: Calculate raw %K
        let mut raw_k = vec![f64::NAN; n];
        for i in (k_period - 1)..n {
            let window = &candles[(i + 1 - k_period)..=i];
            let lowest = window.iter().map(|c| c.low).fold(f64::INFINITY, f64::min);
            let highest = window
                .iter()
                .map(|c| c.high)
                .fold(f64::NEG_INFINITY, f64::max);

            if (highest - lowest).abs() < f64::EPSILON {
                raw_k[i] = 50.0; // Flat range
            } else {
                raw_k[i] = (candles[i].close - lowest) / (highest - lowest) * 100.0;
            }
        }

        // Step 2: Smooth %K with SMA
        let smoothed_k = Self::sma(&raw_k, smooth_k);

        // Step 3: %D = SMA of smoothed %K
        let d_line = Self::sma(&smoothed_k, d_period);

        let mut series = HashMap::new();
        series.insert("k".to_string(), smoothed_k);
        series.insert("d".to_string(), d_line);

        Ok(CalculationOutput::multi(series))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Candle;

    fn make_candle(i: usize, high: f64, low: f64, close: f64) -> Candle {
        Candle {
            timestamp: (i as i64) * 60_000,
            open: (high + low) / 2.0,
            high,
            low,
            close,
            volume: 1000.0,
        }
    }

    #[test]
    fn stochastic_at_highest_is_100() {
        // Price consistently at the top of range
        let mut candles = Vec::new();
        for i in 0..20 {
            candles.push(make_candle(i, 110.0, 100.0, 110.0));
        }
        let input = CalculationInput::new(
            candles,
            HashMap::from([
                ("k_period".into(), ParamValue::Integer(5)),
                ("d_period".into(), ParamValue::Integer(3)),
                ("smooth_k".into(), ParamValue::Integer(3)),
            ]),
        );
        let result = Stochastic.calculate(&input).unwrap();
        let k = result.get("k").unwrap();
        // After warmup, %K should be 100
        let last = k.last().unwrap();
        assert!((last - 100.0).abs() < 1e-10);
    }

    #[test]
    fn stochastic_at_lowest_is_0() {
        let mut candles = Vec::new();
        for i in 0..20 {
            candles.push(make_candle(i, 110.0, 100.0, 100.0));
        }
        let input = CalculationInput::new(
            candles,
            HashMap::from([
                ("k_period".into(), ParamValue::Integer(5)),
                ("d_period".into(), ParamValue::Integer(3)),
                ("smooth_k".into(), ParamValue::Integer(3)),
            ]),
        );
        let result = Stochastic.calculate(&input).unwrap();
        let k = result.get("k").unwrap();
        let last = k.last().unwrap();
        assert!(last.abs() < 1e-10);
    }

    #[test]
    fn stochastic_midpoint_is_50() {
        let mut candles = Vec::new();
        for i in 0..20 {
            candles.push(make_candle(i, 110.0, 100.0, 105.0)); // Midpoint
        }
        let input = CalculationInput::new(
            candles,
            HashMap::from([
                ("k_period".into(), ParamValue::Integer(5)),
                ("d_period".into(), ParamValue::Integer(3)),
                ("smooth_k".into(), ParamValue::Integer(3)),
            ]),
        );
        let result = Stochastic.calculate(&input).unwrap();
        let k = result.get("k").unwrap();
        let last = k.last().unwrap();
        assert!((last - 50.0).abs() < 1e-10);
    }

    #[test]
    fn stochastic_flat_range_returns_50() {
        let mut candles = Vec::new();
        for i in 0..20 {
            candles.push(make_candle(i, 100.0, 100.0, 100.0));
        }
        let input = CalculationInput::new(
            candles,
            HashMap::from([
                ("k_period".into(), ParamValue::Integer(5)),
                ("d_period".into(), ParamValue::Integer(3)),
                ("smooth_k".into(), ParamValue::Integer(3)),
            ]),
        );
        let result = Stochastic.calculate(&input).unwrap();
        let k = result.get("k").unwrap();
        let last = k.last().unwrap();
        assert!((last - 50.0).abs() < 1e-10);
    }

    #[test]
    fn stochastic_warmup_period() {
        let candles: Vec<Candle> = (0..20)
            .map(|i| make_candle(i, 110.0, 100.0, 105.0))
            .collect();
        let input = CalculationInput::new(
            candles,
            HashMap::from([
                ("k_period".into(), ParamValue::Integer(5)),
                ("d_period".into(), ParamValue::Integer(3)),
                ("smooth_k".into(), ParamValue::Integer(3)),
            ]),
        );
        let result = Stochastic.calculate(&input).unwrap();
        let k = result.get("k").unwrap();
        let d = result.get("d").unwrap();
        // raw_k starts at index 4 (k_period-1)
        // smoothed_k starts at index 6 (4 + smooth_k - 1)
        // d starts at index 8 (6 + d_period - 1)
        assert!(k[5].is_nan());
        assert!(!k[6].is_nan());
        assert!(d[7].is_nan());
        assert!(!d[8].is_nan());
    }

    #[test]
    fn stochastic_insufficient_data() {
        let candles: Vec<Candle> = (0..5)
            .map(|i| make_candle(i, 110.0, 100.0, 105.0))
            .collect();
        let input = CalculationInput::new(
            candles,
            HashMap::from([
                ("k_period".into(), ParamValue::Integer(14)),
                ("d_period".into(), ParamValue::Integer(3)),
                ("smooth_k".into(), ParamValue::Integer(3)),
            ]),
        );
        assert!(matches!(
            Stochastic.calculate(&input),
            Err(CalculationError::InsufficientData { .. })
        ));
    }

    #[test]
    fn stochastic_k_and_d_in_range() {
        let candles: Vec<Candle> = (0..30)
            .map(|i| {
                let price = 100.0 + (i as f64 * 0.7).sin() * 10.0;
                make_candle(i, price + 2.0, price - 2.0, price)
            })
            .collect();
        let input = CalculationInput::new(
            candles,
            HashMap::from([
                ("k_period".into(), ParamValue::Integer(5)),
                ("d_period".into(), ParamValue::Integer(3)),
                ("smooth_k".into(), ParamValue::Integer(3)),
            ]),
        );
        let result = Stochastic.calculate(&input).unwrap();
        let k = result.get("k").unwrap();
        let d = result.get("d").unwrap();
        for v in k.iter().chain(d.iter()) {
            if !v.is_nan() {
                assert!(*v >= 0.0 && *v <= 100.0, "Value {} out of range", v);
            }
        }
    }

    #[test]
    fn stochastic_metadata() {
        let meta = Stochastic.metadata();
        assert_eq!(meta.id, "stochastic");
        assert_eq!(meta.parameters.len(), 3);
        assert!(meta.dependencies.is_empty());
        assert_eq!(meta.output_series.len(), 2);
    }
}
