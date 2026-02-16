use crate::input::CalculationInput;
use crate::metadata::*;
use crate::output::*;
use crate::traits::Calculation;
use crate::types::PriceField;

/// On-Balance Volume: cumulative volume indicator that adds volume on up-closes
/// and subtracts volume on down-closes.
pub struct Obv;

impl Calculation for Obv {
    fn metadata(&self) -> CalculationMetadata {
        CalculationMetadata {
            id: "obv".into(),
            title: "On-Balance Volume".into(),
            description: "Cumulative volume indicator. Adds volume when price closes higher, \
                subtracts when price closes lower. Confirms price trends via volume flow."
                .into(),
            parameters: vec![],
            price_fields: vec![PriceField::Close, PriceField::Volume],
            min_candles_description: "2 (need at least one price change)".into(),
            output_series: vec![OutputSeriesDef {
                name: "value".into(),
                description: "Cumulative OBV".into(),
                series_type: SeriesType::Numeric,
            }],
            dependencies: vec![],
        }
    }

    fn min_candles(&self, _input: &CalculationInput) -> usize {
        2
    }

    fn calculate(&self, input: &CalculationInput) -> Result<CalculationOutput, CalculationError> {
        self.validate_input(input)?;
        let n = input.candles.len();
        let mut values = vec![0.0f64; n];

        for i in 1..n {
            let prev_close = input.candles[i - 1].close;
            let close = input.candles[i].close;
            let volume = input.candles[i].volume;

            if close > prev_close {
                values[i] = values[i - 1] + volume;
            } else if close < prev_close {
                values[i] = values[i - 1] - volume;
            } else {
                values[i] = values[i - 1];
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

    fn make_candles(data: &[(f64, f64)]) -> Vec<Candle> {
        data.iter()
            .enumerate()
            .map(|(i, &(close, volume))| Candle {
                timestamp: (i as i64) * 60_000,
                open: close,
                high: close,
                low: close,
                close,
                volume,
            })
            .collect()
    }

    #[test]
    fn obv_uptrend_accumulates() {
        let candles = make_candles(&[
            (100.0, 1000.0),
            (101.0, 1000.0),
            (102.0, 1000.0),
            (103.0, 1000.0),
        ]);
        let input = CalculationInput::new(candles, HashMap::new());
        let result = Obv.calculate(&input).unwrap();
        let values = result.values().unwrap();
        assert_eq!(values, &[0.0, 1000.0, 2000.0, 3000.0]);
    }

    #[test]
    fn obv_downtrend_decreases() {
        let candles = make_candles(&[
            (103.0, 1000.0),
            (102.0, 1000.0),
            (101.0, 1000.0),
            (100.0, 1000.0),
        ]);
        let input = CalculationInput::new(candles, HashMap::new());
        let result = Obv.calculate(&input).unwrap();
        let values = result.values().unwrap();
        assert_eq!(values, &[0.0, -1000.0, -2000.0, -3000.0]);
    }

    #[test]
    fn obv_flat_close_unchanged() {
        let candles = make_candles(&[(100.0, 1000.0), (100.0, 2000.0), (100.0, 3000.0)]);
        let input = CalculationInput::new(candles, HashMap::new());
        let result = Obv.calculate(&input).unwrap();
        let values = result.values().unwrap();
        assert_eq!(values, &[0.0, 0.0, 0.0]);
    }

    #[test]
    fn obv_mixed_direction() {
        let candles = make_candles(&[
            (100.0, 1000.0),
            (101.0, 500.0), // up: +500
            (100.0, 800.0), // down: -800
            (101.0, 600.0), // up: +600
        ]);
        let input = CalculationInput::new(candles, HashMap::new());
        let result = Obv.calculate(&input).unwrap();
        let values = result.values().unwrap();
        assert_eq!(values, &[0.0, 500.0, -300.0, 300.0]);
    }

    #[test]
    fn obv_insufficient_data() {
        let candles = make_candles(&[(100.0, 1000.0)]);
        let input = CalculationInput::new(candles, HashMap::new());
        assert_eq!(
            Obv.calculate(&input).unwrap_err(),
            CalculationError::InsufficientData {
                required: 2,
                provided: 1,
            }
        );
    }

    #[test]
    fn obv_varying_volumes() {
        let candles = make_candles(&[
            (100.0, 100.0),
            (101.0, 1000.0), // up: +1000
            (102.0, 100.0),  // up: +100
        ]);
        let input = CalculationInput::new(candles, HashMap::new());
        let result = Obv.calculate(&input).unwrap();
        let values = result.values().unwrap();
        assert_eq!(values, &[0.0, 1000.0, 1100.0]);
    }

    #[test]
    fn obv_metadata() {
        let meta = Obv.metadata();
        assert_eq!(meta.id, "obv");
        assert!(meta.parameters.is_empty());
        assert!(meta.dependencies.is_empty());
        assert_eq!(meta.output_series.len(), 1);
    }
}
