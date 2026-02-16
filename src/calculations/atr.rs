use crate::input::CalculationInput;
use crate::metadata::*;
use crate::output::*;
use crate::traits::Calculation;
use crate::types::PriceField;

/// Average True Range: volatility indicator using Wilder's smoothing of true range.
pub struct Atr;

impl Atr {
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

impl Calculation for Atr {
    fn metadata(&self) -> CalculationMetadata {
        CalculationMetadata {
            id: "atr".into(),
            title: "Average True Range".into(),
            description:
                "Volatility indicator measuring the average true range of price movement. \
                True Range = max(high-low, |high-prev_close|, |low-prev_close|). \
                Uses Wilder's smoothing method."
                    .into(),
            parameters: vec![ParameterDef {
                name: "period".into(),
                description: "ATR smoothing period".into(),
                param_type: ParameterType::Integer,
                default_value: Some(ParamValue::Integer(14)),
                required: false,
            }],
            price_fields: vec![PriceField::High, PriceField::Low, PriceField::Close],
            min_candles_description:
                "period + 1 (need period true ranges, each requiring a previous close)".into(),
            output_series: vec![OutputSeriesDef {
                name: "value".into(),
                description: "ATR values".into(),
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
        let candles = &input.candles;
        let n = candles.len();
        let mut values = vec![f64::NAN; n];

        // Calculate true range for each candle starting at index 1
        let mut tr = vec![0.0f64; n];
        tr[0] = candles[0].high - candles[0].low; // No previous close for first candle
        for i in 1..n {
            let hl = candles[i].high - candles[i].low;
            let hpc = (candles[i].high - candles[i - 1].close).abs();
            let lpc = (candles[i].low - candles[i - 1].close).abs();
            tr[i] = hl.max(hpc).max(lpc);
        }

        // Initial ATR: simple average of first `period` true ranges (indices 1..=period)
        let initial_sum: f64 = tr[1..=period].iter().sum();
        values[period] = initial_sum / period as f64;

        // Wilder's smoothing for subsequent values
        for i in (period + 1)..n {
            values[i] = (values[i - 1] * (period as f64 - 1.0) + tr[i]) / period as f64;
        }

        Ok(CalculationOutput::single(values))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Candle;
    use std::collections::HashMap;

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
    fn atr_constant_range() {
        // All candles have range of 2.0, no gaps
        let candles: Vec<Candle> = (0..20)
            .map(|i| make_candle(i, 101.0, 99.0, 100.0))
            .collect();
        let input = CalculationInput::new(
            candles,
            HashMap::from([("period".into(), ParamValue::Integer(14))]),
        );
        let result = Atr.calculate(&input).unwrap();
        let values = result.values().unwrap();
        assert!(values[0..14].iter().all(|v| v.is_nan()));
        // Constant range means ATR should converge to 2.0
        assert!((values[14] - 2.0).abs() < 1e-10);
        assert!((values[19] - 2.0).abs() < 1e-10);
    }

    #[test]
    fn atr_with_gaps() {
        // Create candles where the gap dominates the range
        let mut candles = vec![make_candle(0, 101.0, 99.0, 100.0)];
        // Next candle gaps up: prev_close=100, high=115, low=110
        // TR = max(115-110, |115-100|, |110-100|) = max(5, 15, 10) = 15
        candles.push(make_candle(1, 115.0, 110.0, 112.0));
        // Fill remaining with constant range
        for i in 2..20 {
            candles.push(make_candle(i, 113.0, 111.0, 112.0));
        }
        let input = CalculationInput::new(
            candles,
            HashMap::from([("period".into(), ParamValue::Integer(3))]),
        );
        let result = Atr.calculate(&input).unwrap();
        let values = result.values().unwrap();
        // Initial ATR at index 3: avg of TR[1..=3]
        // TR[1] = 15, TR[2] = 2, TR[3] = 2 => avg = 19/3 ≈ 6.333
        assert!((values[3] - 19.0 / 3.0).abs() < 1e-10);
    }

    #[test]
    fn atr_small_period() {
        let candles: Vec<Candle> = (0..5).map(|i| make_candle(i, 101.0, 99.0, 100.0)).collect();
        let input = CalculationInput::new(
            candles,
            HashMap::from([("period".into(), ParamValue::Integer(3))]),
        );
        let result = Atr.calculate(&input).unwrap();
        let values = result.values().unwrap();
        assert!(values[0..3].iter().all(|v| v.is_nan()));
        assert!(!values[3].is_nan());
    }

    #[test]
    fn atr_insufficient_data() {
        let candles: Vec<Candle> = (0..14)
            .map(|i| make_candle(i, 101.0, 99.0, 100.0))
            .collect();
        let input = CalculationInput::new(
            candles,
            HashMap::from([("period".into(), ParamValue::Integer(14))]),
        );
        assert_eq!(
            Atr.calculate(&input).unwrap_err(),
            CalculationError::InsufficientData {
                required: 15,
                provided: 14,
            }
        );
    }

    #[test]
    fn atr_invalid_period() {
        let candles: Vec<Candle> = (0..20)
            .map(|i| make_candle(i, 101.0, 99.0, 100.0))
            .collect();
        let input = CalculationInput::new(
            candles,
            HashMap::from([("period".into(), ParamValue::Integer(-1))]),
        );
        assert!(matches!(
            Atr.calculate(&input),
            Err(CalculationError::InvalidParameter { .. })
        ));
    }

    #[test]
    fn atr_warmup_is_nan() {
        let candles: Vec<Candle> = (0..20)
            .map(|i| make_candle(i, 101.0, 99.0, 100.0))
            .collect();
        let input = CalculationInput::new(
            candles,
            HashMap::from([("period".into(), ParamValue::Integer(14))]),
        );
        let result = Atr.calculate(&input).unwrap();
        let values = result.values().unwrap();
        assert!(values[0..14].iter().all(|v| v.is_nan()));
        assert!(!values[14].is_nan());
    }

    #[test]
    fn atr_metadata() {
        let meta = Atr.metadata();
        assert_eq!(meta.id, "atr");
        assert_eq!(meta.parameters.len(), 1);
        assert!(meta.dependencies.is_empty());
        assert_eq!(meta.output_series.len(), 1);
    }
}
