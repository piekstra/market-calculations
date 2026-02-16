use std::collections::HashMap;

use crate::input::CalculationInput;
use crate::metadata::*;
use crate::output::*;
use crate::traits::Calculation;
use crate::types::PriceField;

/// Volume-Weighted Average Price with standard deviation bands.
/// Resets at the start of each trading day.
pub struct Vwap;

impl Calculation for Vwap {
    fn metadata(&self) -> CalculationMetadata {
        CalculationMetadata {
            id: "vwap".into(),
            title: "Volume-Weighted Average Price".into(),
            description: "Intraday indicator showing average price weighted by volume. \
                Resets each trading day. Typical price = (high + low + close) / 3. \
                Bands show ±1 standard deviation from VWAP."
                .into(),
            parameters: vec![],
            price_fields: vec![PriceField::FullCandle],
            min_candles_description: "2 (need at least 2 candles to calculate)".into(),
            output_series: vec![
                OutputSeriesDef {
                    name: "value".into(),
                    description: "VWAP".into(),
                    series_type: SeriesType::Numeric,
                },
                OutputSeriesDef {
                    name: "upper_band".into(),
                    description: "VWAP + 1 standard deviation".into(),
                    series_type: SeriesType::Numeric,
                },
                OutputSeriesDef {
                    name: "lower_band".into(),
                    description: "VWAP - 1 standard deviation".into(),
                    series_type: SeriesType::Numeric,
                },
            ],
            dependencies: vec![],
        }
    }

    fn min_candles(&self, _input: &CalculationInput) -> usize {
        2
    }

    fn calculate(&self, input: &CalculationInput) -> Result<CalculationOutput, CalculationError> {
        self.validate_input(input)?;
        let candles = &input.candles;
        let n = candles.len();

        let mut vwap_values = vec![f64::NAN; n];
        let mut upper_band = vec![f64::NAN; n];
        let mut lower_band = vec![f64::NAN; n];

        let ms_per_day = 24 * 60 * 60 * 1000_i64;
        let mut cum_tp_vol = 0.0f64;
        let mut cum_vol = 0.0f64;
        let mut day_start_idx = 0usize;
        let mut current_day: Option<i64> = None;

        for i in 0..n {
            let day = candles[i].timestamp / ms_per_day;

            // Reset on new day
            if current_day != Some(day) {
                cum_tp_vol = 0.0;
                cum_vol = 0.0;
                day_start_idx = i;
                current_day = Some(day);
            }

            let tp = (candles[i].high + candles[i].low + candles[i].close) / 3.0;
            let vol = candles[i].volume;

            cum_tp_vol += tp * vol;
            cum_vol += vol;

            if cum_vol == 0.0 {
                continue;
            }

            let vwap = cum_tp_vol / cum_vol;
            vwap_values[i] = vwap;

            let candles_in_day = i - day_start_idx + 1;

            // Volume-weighted standard deviation
            if candles_in_day >= 2 {
                let mut weighted_var_sum = 0.0f64;
                let mut vol_sum = 0.0f64;
                for candle in &candles[day_start_idx..=i] {
                    let tp_j = (candle.high + candle.low + candle.close) / 3.0;
                    let v = candle.volume;
                    weighted_var_sum += v * (tp_j - vwap).powi(2);
                    vol_sum += v;
                }
                let std_dev = if vol_sum > 0.0 {
                    (weighted_var_sum / vol_sum).sqrt()
                } else {
                    0.0
                };
                upper_band[i] = vwap + std_dev;
                lower_band[i] = vwap - std_dev;
            } else {
                upper_band[i] = vwap;
                lower_band[i] = vwap;
            }
        }

        let mut series = HashMap::new();
        series.insert("value".to_string(), vwap_values);
        series.insert("upper_band".to_string(), upper_band);
        series.insert("lower_band".to_string(), lower_band);

        Ok(CalculationOutput::multi(series))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Candle;

    fn make_candle(timestamp: i64, high: f64, low: f64, close: f64, volume: f64) -> Candle {
        Candle {
            timestamp,
            open: (high + low) / 2.0,
            high,
            low,
            close,
            volume,
        }
    }

    #[test]
    fn vwap_equal_volumes_equals_average_tp() {
        let candles = vec![
            make_candle(0, 102.0, 98.0, 100.0, 1000.0),
            make_candle(60_000, 104.0, 100.0, 102.0, 1000.0),
            make_candle(120_000, 106.0, 102.0, 104.0, 1000.0),
        ];
        let input = CalculationInput::new(candles, HashMap::new());
        let result = Vwap.calculate(&input).unwrap();
        let values = result.get("value").unwrap();

        // TP1 = (102+98+100)/3 = 100.0
        assert!((values[0] - 100.0).abs() < 1e-10);
        // TP2 = (104+100+102)/3 = 102.0, VWAP = (100*1000+102*1000)/2000 = 101.0
        assert!((values[1] - 101.0).abs() < 1e-10);
        // TP3 = (106+102+104)/3 = 104.0, VWAP = (100+102+104)*1000/3000 = 102.0
        assert!((values[2] - 102.0).abs() < 1e-10);
    }

    #[test]
    fn vwap_weighted_toward_higher_volume() {
        let candles = vec![
            make_candle(0, 100.0, 100.0, 100.0, 10000.0), // TP=100, heavy vol
            make_candle(60_000, 110.0, 110.0, 110.0, 100.0), // TP=110, light vol
        ];
        let input = CalculationInput::new(candles, HashMap::new());
        let result = Vwap.calculate(&input).unwrap();
        let values = result.get("value").unwrap();

        // VWAP = (100*10000 + 110*100) / (10000+100) = 1011000/10100 ≈ 100.099
        let expected = (100.0 * 10000.0 + 110.0 * 100.0) / (10000.0 + 100.0);
        assert!((values[1] - expected).abs() < 1e-10);
    }

    #[test]
    fn vwap_resets_on_new_day() {
        let ms_per_day = 24 * 60 * 60 * 1000_i64;
        let candles = vec![
            make_candle(0, 100.0, 100.0, 100.0, 1000.0),      // Day 0
            make_candle(60_000, 102.0, 102.0, 102.0, 1000.0), // Day 0
            make_candle(ms_per_day, 200.0, 200.0, 200.0, 1000.0), // Day 1 (reset)
            make_candle(ms_per_day + 60_000, 202.0, 202.0, 202.0, 1000.0), // Day 1
        ];
        let input = CalculationInput::new(candles, HashMap::new());
        let result = Vwap.calculate(&input).unwrap();
        let values = result.get("value").unwrap();

        // Day 1 first candle: VWAP should be 200.0 (reset)
        assert!((values[2] - 200.0).abs() < 1e-10);
        // Day 1 second candle: VWAP = (200+202)/2 = 201.0
        assert!((values[3] - 201.0).abs() < 1e-10);
    }

    #[test]
    fn vwap_bands_contain_price() {
        let candles = vec![
            make_candle(0, 102.0, 98.0, 100.0, 1000.0),
            make_candle(60_000, 108.0, 96.0, 102.0, 1000.0),
            make_candle(120_000, 110.0, 94.0, 104.0, 1000.0),
        ];
        let input = CalculationInput::new(candles, HashMap::new());
        let result = Vwap.calculate(&input).unwrap();
        let upper = result.get("upper_band").unwrap();
        let lower = result.get("lower_band").unwrap();
        let values = result.get("value").unwrap();

        for i in 1..3 {
            assert!(upper[i] >= values[i]);
            assert!(lower[i] <= values[i]);
        }
    }

    #[test]
    fn vwap_insufficient_data() {
        let candles = vec![make_candle(0, 100.0, 100.0, 100.0, 1000.0)];
        let input = CalculationInput::new(candles, HashMap::new());
        assert_eq!(
            Vwap.calculate(&input).unwrap_err(),
            CalculationError::InsufficientData {
                required: 2,
                provided: 1,
            }
        );
    }

    #[test]
    fn vwap_metadata() {
        let meta = Vwap.metadata();
        assert_eq!(meta.id, "vwap");
        assert!(meta.parameters.is_empty());
        assert!(meta.dependencies.is_empty());
        assert_eq!(meta.output_series.len(), 3);
        assert_eq!(meta.price_fields, vec![PriceField::FullCandle]);
    }
}
