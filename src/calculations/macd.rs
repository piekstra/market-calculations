use std::collections::HashMap;

use crate::input::CalculationInput;
use crate::metadata::*;
use crate::output::*;
use crate::traits::Calculation;
use crate::types::PriceField;

/// MACD (Moving Average Convergence Divergence): trend-following momentum indicator.
/// Computes EMA internally rather than depending on the EMA calculation.
pub struct Macd;

impl Macd {
    fn get_fast_period(input: &CalculationInput) -> Result<usize, CalculationError> {
        match input.params.get("fast_period") {
            Some(ParamValue::Integer(p)) if *p > 0 => Ok(*p as usize),
            Some(ParamValue::Integer(p)) => Err(CalculationError::InvalidParameter {
                name: "fast_period".into(),
                reason: format!("fast_period must be positive, got {}", p),
            }),
            Some(_) => Err(CalculationError::InvalidParameter {
                name: "fast_period".into(),
                reason: "fast_period must be an integer".into(),
            }),
            None => Ok(12),
        }
    }

    fn get_slow_period(input: &CalculationInput) -> Result<usize, CalculationError> {
        match input.params.get("slow_period") {
            Some(ParamValue::Integer(p)) if *p > 0 => Ok(*p as usize),
            Some(ParamValue::Integer(p)) => Err(CalculationError::InvalidParameter {
                name: "slow_period".into(),
                reason: format!("slow_period must be positive, got {}", p),
            }),
            Some(_) => Err(CalculationError::InvalidParameter {
                name: "slow_period".into(),
                reason: "slow_period must be an integer".into(),
            }),
            None => Ok(26),
        }
    }

    fn get_signal_period(input: &CalculationInput) -> Result<usize, CalculationError> {
        match input.params.get("signal_period") {
            Some(ParamValue::Integer(p)) if *p > 0 => Ok(*p as usize),
            Some(ParamValue::Integer(p)) => Err(CalculationError::InvalidParameter {
                name: "signal_period".into(),
                reason: format!("signal_period must be positive, got {}", p),
            }),
            Some(_) => Err(CalculationError::InvalidParameter {
                name: "signal_period".into(),
                reason: "signal_period must be an integer".into(),
            }),
            None => Ok(9),
        }
    }

    /// Compute EMA from a slice of f64 values. Returns NaN for warmup positions.
    fn compute_ema(data: &[f64], period: usize) -> Vec<f64> {
        let n = data.len();
        let mut ema = vec![f64::NAN; n];
        if n < period {
            return ema;
        }

        // Initial SMA for seed
        let mut sum = 0.0f64;
        let mut count = 0;
        for val in &data[..period] {
            if !val.is_nan() {
                sum += val;
                count += 1;
            }
        }
        if count == 0 {
            return ema;
        }
        ema[period - 1] = sum / count as f64;

        // EMA with multiplier
        let multiplier = 2.0 / (period as f64 + 1.0);
        for i in period..n {
            if data[i].is_nan() {
                ema[i] = ema[i - 1];
            } else {
                ema[i] = (data[i] - ema[i - 1]) * multiplier + ema[i - 1];
            }
        }
        ema
    }
}

impl Calculation for Macd {
    fn metadata(&self) -> CalculationMetadata {
        CalculationMetadata {
            id: "macd".into(),
            title: "Moving Average Convergence Divergence".into(),
            description: "Trend-following momentum indicator. MACD line = EMA(fast) - EMA(slow). \
                Signal line = EMA of MACD line. Histogram = MACD - Signal."
                .into(),
            parameters: vec![
                ParameterDef {
                    name: "fast_period".into(),
                    description: "Fast EMA period".into(),
                    param_type: ParameterType::Integer,
                    default_value: Some(ParamValue::Integer(12)),
                    required: false,
                },
                ParameterDef {
                    name: "slow_period".into(),
                    description: "Slow EMA period".into(),
                    param_type: ParameterType::Integer,
                    default_value: Some(ParamValue::Integer(26)),
                    required: false,
                },
                ParameterDef {
                    name: "signal_period".into(),
                    description: "Signal line EMA period".into(),
                    param_type: ParameterType::Integer,
                    default_value: Some(ParamValue::Integer(9)),
                    required: false,
                },
            ],
            price_fields: vec![PriceField::Close],
            min_candles_description: "slow_period + signal_period - 1".into(),
            output_series: vec![
                OutputSeriesDef {
                    name: "macd_line".into(),
                    description: "MACD line (fast EMA - slow EMA)".into(),
                    series_type: SeriesType::Numeric,
                },
                OutputSeriesDef {
                    name: "signal_line".into(),
                    description: "Signal line (EMA of MACD)".into(),
                    series_type: SeriesType::Numeric,
                },
                OutputSeriesDef {
                    name: "histogram".into(),
                    description: "MACD histogram (MACD - signal)".into(),
                    series_type: SeriesType::Numeric,
                },
            ],
            dependencies: vec![],
        }
    }

    fn min_candles(&self, input: &CalculationInput) -> usize {
        let slow = Self::get_slow_period(input).unwrap_or(26);
        let signal = Self::get_signal_period(input).unwrap_or(9);
        slow + signal - 1
    }

    fn calculate(&self, input: &CalculationInput) -> Result<CalculationOutput, CalculationError> {
        self.validate_input(input)?;
        let fast_period = Self::get_fast_period(input)?;
        let slow_period = Self::get_slow_period(input)?;
        let signal_period = Self::get_signal_period(input)?;

        if fast_period >= slow_period {
            return Err(CalculationError::InvalidParameter {
                name: "fast_period".into(),
                reason: format!(
                    "fast_period ({}) must be less than slow_period ({})",
                    fast_period, slow_period
                ),
            });
        }

        let closes: Vec<f64> = input.candles.iter().map(|c| c.close).collect();
        let n = closes.len();

        // Compute fast and slow EMAs
        let fast_ema = Self::compute_ema(&closes, fast_period);
        let slow_ema = Self::compute_ema(&closes, slow_period);

        // MACD line = fast EMA - slow EMA
        let mut macd_line = vec![f64::NAN; n];
        for i in 0..n {
            if !fast_ema[i].is_nan() && !slow_ema[i].is_nan() {
                macd_line[i] = fast_ema[i] - slow_ema[i];
            }
        }

        // Signal line = EMA of MACD line (starting from first valid MACD value)
        // We need to find where MACD starts being valid and compute EMA from there
        let first_valid = macd_line.iter().position(|v| !v.is_nan()).unwrap_or(n);
        let mut signal_line = vec![f64::NAN; n];

        if first_valid + signal_period <= n {
            // Seed with SMA of first signal_period MACD values
            let seed_sum: f64 = macd_line[first_valid..first_valid + signal_period]
                .iter()
                .sum();
            let seed_idx = first_valid + signal_period - 1;
            signal_line[seed_idx] = seed_sum / signal_period as f64;

            let multiplier = 2.0 / (signal_period as f64 + 1.0);
            for i in (seed_idx + 1)..n {
                if !macd_line[i].is_nan() {
                    signal_line[i] =
                        (macd_line[i] - signal_line[i - 1]) * multiplier + signal_line[i - 1];
                }
            }
        }

        // Histogram = MACD - Signal
        let mut histogram = vec![f64::NAN; n];
        for i in 0..n {
            if !macd_line[i].is_nan() && !signal_line[i].is_nan() {
                histogram[i] = macd_line[i] - signal_line[i];
            }
        }

        let mut series = HashMap::new();
        series.insert("macd_line".to_string(), macd_line);
        series.insert("signal_line".to_string(), signal_line);
        series.insert("histogram".to_string(), histogram);

        Ok(CalculationOutput::multi(series))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Candle;

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
    fn macd_constant_prices_near_zero() {
        let closes = vec![100.0; 40];
        let input = CalculationInput::new(
            make_candles(&closes),
            HashMap::from([
                ("fast_period".into(), ParamValue::Integer(12)),
                ("slow_period".into(), ParamValue::Integer(26)),
                ("signal_period".into(), ParamValue::Integer(9)),
            ]),
        );
        let result = Macd.calculate(&input).unwrap();
        let macd = result.get("macd_line").unwrap();
        let signal = result.get("signal_line").unwrap();
        let hist = result.get("histogram").unwrap();

        // With constant prices, MACD should be near zero
        let last = macd.last().unwrap();
        assert!(last.abs() < 1e-10, "MACD should be ~0, got {}", last);
        assert!(signal.last().unwrap().abs() < 1e-10);
        assert!(hist.last().unwrap().abs() < 1e-10);
    }

    #[test]
    fn macd_uptrend_positive() {
        let closes: Vec<f64> = (0..40).map(|i| 100.0 + i as f64).collect();
        let input = CalculationInput::new(
            make_candles(&closes),
            HashMap::from([
                ("fast_period".into(), ParamValue::Integer(12)),
                ("slow_period".into(), ParamValue::Integer(26)),
                ("signal_period".into(), ParamValue::Integer(9)),
            ]),
        );
        let result = Macd.calculate(&input).unwrap();
        let macd = result.get("macd_line").unwrap();

        // In uptrend, fast EMA > slow EMA, so MACD should be positive
        let last = macd.last().unwrap();
        assert!(
            *last > 0.0,
            "MACD should be positive in uptrend, got {}",
            last
        );
    }

    #[test]
    fn macd_downtrend_negative() {
        let closes: Vec<f64> = (0..40).map(|i| 200.0 - i as f64).collect();
        let input = CalculationInput::new(
            make_candles(&closes),
            HashMap::from([
                ("fast_period".into(), ParamValue::Integer(12)),
                ("slow_period".into(), ParamValue::Integer(26)),
                ("signal_period".into(), ParamValue::Integer(9)),
            ]),
        );
        let result = Macd.calculate(&input).unwrap();
        let macd = result.get("macd_line").unwrap();

        let last = macd.last().unwrap();
        assert!(
            *last < 0.0,
            "MACD should be negative in downtrend, got {}",
            last
        );
    }

    #[test]
    fn macd_histogram_equals_macd_minus_signal() {
        let closes: Vec<f64> = (0..40)
            .map(|i| 100.0 + (i as f64 * 0.5).sin() * 10.0)
            .collect();
        let input = CalculationInput::new(
            make_candles(&closes),
            HashMap::from([
                ("fast_period".into(), ParamValue::Integer(12)),
                ("slow_period".into(), ParamValue::Integer(26)),
                ("signal_period".into(), ParamValue::Integer(9)),
            ]),
        );
        let result = Macd.calculate(&input).unwrap();
        let macd = result.get("macd_line").unwrap();
        let signal = result.get("signal_line").unwrap();
        let hist = result.get("histogram").unwrap();

        for i in 0..40 {
            if !macd[i].is_nan() && !signal[i].is_nan() {
                assert!(
                    (hist[i] - (macd[i] - signal[i])).abs() < 1e-10,
                    "Histogram mismatch at index {}",
                    i
                );
            }
        }
    }

    #[test]
    fn macd_warmup_is_nan() {
        let closes: Vec<f64> = (0..40).map(|i| 100.0 + i as f64).collect();
        let input = CalculationInput::new(
            make_candles(&closes),
            HashMap::from([
                ("fast_period".into(), ParamValue::Integer(12)),
                ("slow_period".into(), ParamValue::Integer(26)),
                ("signal_period".into(), ParamValue::Integer(9)),
            ]),
        );
        let result = Macd.calculate(&input).unwrap();
        let macd = result.get("macd_line").unwrap();
        let signal = result.get("signal_line").unwrap();

        // MACD line starts at slow_period - 1 = 25
        assert!(macd[24].is_nan());
        assert!(!macd[25].is_nan());

        // Signal line starts at 25 + signal_period - 1 = 33
        assert!(signal[32].is_nan());
        assert!(!signal[33].is_nan());
    }

    #[test]
    fn macd_fast_gte_slow_errors() {
        let closes = vec![100.0; 40];
        let input = CalculationInput::new(
            make_candles(&closes),
            HashMap::from([
                ("fast_period".into(), ParamValue::Integer(26)),
                ("slow_period".into(), ParamValue::Integer(12)),
                ("signal_period".into(), ParamValue::Integer(9)),
            ]),
        );
        assert!(matches!(
            Macd.calculate(&input),
            Err(CalculationError::InvalidParameter { .. })
        ));
    }

    #[test]
    fn macd_insufficient_data() {
        let closes = vec![100.0; 30];
        let input = CalculationInput::new(
            make_candles(&closes),
            HashMap::from([
                ("fast_period".into(), ParamValue::Integer(12)),
                ("slow_period".into(), ParamValue::Integer(26)),
                ("signal_period".into(), ParamValue::Integer(9)),
            ]),
        );
        // min_candles = 26 + 9 - 1 = 34, we only have 30
        assert_eq!(
            Macd.calculate(&input).unwrap_err(),
            CalculationError::InsufficientData {
                required: 34,
                provided: 30,
            }
        );
    }

    #[test]
    fn macd_small_periods() {
        let closes: Vec<f64> = (0..15).map(|i| 100.0 + i as f64).collect();
        let input = CalculationInput::new(
            make_candles(&closes),
            HashMap::from([
                ("fast_period".into(), ParamValue::Integer(3)),
                ("slow_period".into(), ParamValue::Integer(5)),
                ("signal_period".into(), ParamValue::Integer(2)),
            ]),
        );
        let result = Macd.calculate(&input).unwrap();
        let macd = result.get("macd_line").unwrap();
        // MACD starts at slow_period - 1 = 4
        assert!(!macd[4].is_nan());
        assert!(*macd.last().unwrap() > 0.0);
    }

    #[test]
    fn macd_metadata() {
        let meta = Macd.metadata();
        assert_eq!(meta.id, "macd");
        assert_eq!(meta.parameters.len(), 3);
        assert!(meta.dependencies.is_empty());
        assert_eq!(meta.output_series.len(), 3);
    }
}
