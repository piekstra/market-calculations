use std::collections::HashMap;

use crate::input::CalculationInput;
use crate::metadata::*;
use crate::output::*;
use crate::traits::Calculation;
use crate::types::{Candle, PriceField};

/// Analyzes daily price behavior from intraday candles.
/// Produces multiple series, one value per trading day.
pub struct DailyProfile;

struct TradingDay {
    candles: Vec<Candle>,
}

impl TradingDay {
    /// Daily range as a percentage of the open.
    fn range_pct(&self) -> f64 {
        let high = self
            .candles
            .iter()
            .map(|c| c.high)
            .fold(f64::NEG_INFINITY, f64::max);
        let low = self
            .candles
            .iter()
            .map(|c| c.low)
            .fold(f64::INFINITY, f64::min);
        let open = self.candles.first().unwrap().open;
        if open == 0.0 {
            return f64::NAN;
        }
        (high - low) / open * 100.0
    }

    /// Did the daily high occur before the daily low? 1.0 = yes, 0.0 = no.
    fn high_before_low(&self) -> f64 {
        let (high_idx, low_idx) = self.high_low_indices();
        if high_idx <= low_idx { 1.0 } else { 0.0 }
    }

    /// Number of candle intervals between the daily high and low.
    fn tightness(&self) -> f64 {
        let (high_idx, low_idx) = self.high_low_indices();
        (high_idx as i64 - low_idx as i64).unsigned_abs() as f64
    }

    /// Close-to-open gap as percentage: (today_open - prev_close) / prev_close * 100.
    fn gap_pct(&self, prev_close: Option<f64>) -> f64 {
        match prev_close {
            Some(pc) if pc != 0.0 => {
                let open = self.candles.first().unwrap().open;
                (open - pc) / pc * 100.0
            }
            _ => f64::NAN,
        }
    }

    /// Intraday return: (close - open) / open * 100.
    fn open_to_close_pct(&self) -> f64 {
        let open = self.candles.first().unwrap().open;
        let close = self.candles.last().unwrap().close;
        if open == 0.0 {
            return f64::NAN;
        }
        (close - open) / open * 100.0
    }

    fn close(&self) -> f64 {
        self.candles.last().unwrap().close
    }

    fn high_low_indices(&self) -> (usize, usize) {
        let mut high_idx = 0;
        let mut low_idx = 0;
        let mut high_val = f64::NEG_INFINITY;
        let mut low_val = f64::INFINITY;
        for (i, c) in self.candles.iter().enumerate() {
            if c.high > high_val {
                high_val = c.high;
                high_idx = i;
            }
            if c.low < low_val {
                low_val = c.low;
                low_idx = i;
            }
        }
        (high_idx, low_idx)
    }
}

impl DailyProfile {
    /// Groups candles by calendar day using UTC (timestamp / ms_per_day).
    fn group_by_day(candles: &[Candle]) -> Vec<TradingDay> {
        let ms_per_day = 24 * 60 * 60 * 1000_i64;
        let mut days: Vec<TradingDay> = Vec::new();
        let mut current_day_candles: Vec<Candle> = Vec::new();
        let mut current_date: Option<i64> = None;

        for candle in candles {
            let day = candle.timestamp / ms_per_day;
            if current_date != Some(day) {
                if !current_day_candles.is_empty() {
                    days.push(TradingDay {
                        candles: std::mem::take(&mut current_day_candles),
                    });
                }
                current_date = Some(day);
            }
            current_day_candles.push(candle.clone());
        }
        if !current_day_candles.is_empty() {
            days.push(TradingDay {
                candles: current_day_candles,
            });
        }
        days
    }
}

impl Calculation for DailyProfile {
    fn metadata(&self) -> CalculationMetadata {
        CalculationMetadata {
            id: "daily_profile".into(),
            title: "Daily Price Profile".into(),
            description: "Analyzes intraday candles to produce per-day metrics: \
                range, high-before-low ordering, tightness, gap%, and intraday return."
                .into(),
            parameters: vec![],
            price_fields: vec![PriceField::FullCandle],
            min_candles_description: "At least 2 trading days of intraday candles".into(),
            output_series: vec![
                OutputSeriesDef {
                    name: "daily_range_pct".into(),
                    description: "Daily high-low range as % of open".into(),
                    series_type: SeriesType::Percentage,
                },
                OutputSeriesDef {
                    name: "high_before_low".into(),
                    description: "1.0 if daily high occurred before daily low, else 0.0".into(),
                    series_type: SeriesType::Boolean,
                },
                OutputSeriesDef {
                    name: "tightness".into(),
                    description: "Number of candle intervals between daily high and low".into(),
                    series_type: SeriesType::Numeric,
                },
                OutputSeriesDef {
                    name: "gap_pct".into(),
                    description: "Open-to-previous-close gap as percentage".into(),
                    series_type: SeriesType::Percentage,
                },
                OutputSeriesDef {
                    name: "open_to_close_pct".into(),
                    description: "Intraday return (close - open) / open as percentage".into(),
                    series_type: SeriesType::Percentage,
                },
            ],
            dependencies: vec![],
        }
    }

    fn min_candles(&self, _input: &CalculationInput) -> usize {
        2
    }

    fn calculate(&self, input: &CalculationInput) -> Result<CalculationOutput, CalculationError> {
        let days = Self::group_by_day(&input.candles);
        if days.len() < 2 {
            return Err(CalculationError::InsufficientData {
                required: 2,
                provided: days.len(),
            });
        }

        let n = days.len();
        let mut daily_range_pct = Vec::with_capacity(n);
        let mut high_before_low = Vec::with_capacity(n);
        let mut tightness = Vec::with_capacity(n);
        let mut gap_pct = Vec::with_capacity(n);
        let mut open_to_close_pct = Vec::with_capacity(n);

        let mut prev_close: Option<f64> = None;
        for day in &days {
            daily_range_pct.push(day.range_pct());
            high_before_low.push(day.high_before_low());
            tightness.push(day.tightness());
            gap_pct.push(day.gap_pct(prev_close));
            open_to_close_pct.push(day.open_to_close_pct());
            prev_close = Some(day.close());
        }

        let mut series = HashMap::new();
        series.insert("daily_range_pct".to_string(), daily_range_pct);
        series.insert("high_before_low".to_string(), high_before_low);
        series.insert("tightness".to_string(), tightness);
        series.insert("gap_pct".to_string(), gap_pct);
        series.insert("open_to_close_pct".to_string(), open_to_close_pct);

        Ok(CalculationOutput::multi(series))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_intraday_candles() -> Vec<Candle> {
        let ms_per_day = 24 * 60 * 60 * 1000_i64;
        let mut candles = Vec::new();

        // Day 1: open=100, prices go up. High at end, low at start.
        // high_before_low = 0.0 (high comes after low)
        candles.push(Candle {
            timestamp: ms_per_day * 0,
            open: 100.0,
            high: 101.0,
            low: 98.0,
            close: 100.5,
            volume: 1000.0,
        });
        candles.push(Candle {
            timestamp: ms_per_day * 0 + 60_000,
            open: 100.5,
            high: 105.0,
            low: 100.0,
            close: 104.0,
            volume: 1000.0,
        });

        // Day 2: open=106, prices go down. High at start, low at end.
        // high_before_low = 1.0
        candles.push(Candle {
            timestamp: ms_per_day * 1,
            open: 106.0,
            high: 108.0,
            low: 105.0,
            close: 107.0,
            volume: 1000.0,
        });
        candles.push(Candle {
            timestamp: ms_per_day * 1 + 60_000,
            open: 107.0,
            high: 107.5,
            low: 102.0,
            close: 103.0,
            volume: 1000.0,
        });

        // Day 3: flat day
        candles.push(Candle {
            timestamp: ms_per_day * 2,
            open: 103.0,
            high: 104.0,
            low: 102.0,
            close: 103.5,
            volume: 1000.0,
        });
        candles.push(Candle {
            timestamp: ms_per_day * 2 + 60_000,
            open: 103.5,
            high: 104.5,
            low: 103.0,
            close: 104.0,
            volume: 1000.0,
        });

        candles
    }

    #[test]
    fn daily_profile_produces_all_series() {
        let input = CalculationInput::new(make_intraday_candles(), HashMap::new());
        let result = DailyProfile.calculate(&input).unwrap();
        assert!(result.get("daily_range_pct").is_some());
        assert!(result.get("high_before_low").is_some());
        assert!(result.get("tightness").is_some());
        assert!(result.get("gap_pct").is_some());
        assert!(result.get("open_to_close_pct").is_some());
    }

    #[test]
    fn daily_profile_correct_day_count() {
        let input = CalculationInput::new(make_intraday_candles(), HashMap::new());
        let result = DailyProfile.calculate(&input).unwrap();
        assert_eq!(result.get("daily_range_pct").unwrap().len(), 3);
    }

    #[test]
    fn daily_profile_high_before_low() {
        let input = CalculationInput::new(make_intraday_candles(), HashMap::new());
        let result = DailyProfile.calculate(&input).unwrap();
        let hbl = result.get("high_before_low").unwrap();
        // Day 1: high at candle 1 (idx=1), low at candle 0 (idx=0). high_idx=1 > low_idx=0 -> 0.0
        assert!((hbl[0] - 0.0).abs() < 1e-10);
        // Day 2: high at candle 0 (idx=0, high=108), low at candle 1 (idx=1, low=102). high_idx=0 <= low_idx=1 -> 1.0
        assert!((hbl[1] - 1.0).abs() < 1e-10);
    }

    #[test]
    fn daily_profile_gap_pct() {
        let input = CalculationInput::new(make_intraday_candles(), HashMap::new());
        let result = DailyProfile.calculate(&input).unwrap();
        let gaps = result.get("gap_pct").unwrap();
        // Day 1: no previous close -> NaN
        assert!(gaps[0].is_nan());
        // Day 2: prev_close=104.0 (day 1 last candle), open=106.0
        // gap = (106.0 - 104.0) / 104.0 * 100 = 1.923...
        assert!((gaps[1] - (106.0 - 104.0) / 104.0 * 100.0).abs() < 1e-10);
    }

    #[test]
    fn daily_profile_range_pct() {
        let input = CalculationInput::new(make_intraday_candles(), HashMap::new());
        let result = DailyProfile.calculate(&input).unwrap();
        let ranges = result.get("daily_range_pct").unwrap();
        // Day 1: high=105 (candle 1), low=98 (candle 0), open=100
        // range = (105 - 98) / 100 * 100 = 7.0%
        assert!((ranges[0] - 7.0).abs() < 1e-10);
    }

    #[test]
    fn daily_profile_open_to_close_pct() {
        let input = CalculationInput::new(make_intraday_candles(), HashMap::new());
        let result = DailyProfile.calculate(&input).unwrap();
        let otc = result.get("open_to_close_pct").unwrap();
        // Day 1: open=100, close=104 (last candle), (104-100)/100*100 = 4.0%
        assert!((otc[0] - 4.0).abs() < 1e-10);
    }

    #[test]
    fn daily_profile_insufficient_data_single_day() {
        let candles = vec![Candle {
            timestamp: 0,
            open: 100.0,
            high: 101.0,
            low: 99.0,
            close: 100.5,
            volume: 1000.0,
        }];
        let input = CalculationInput::new(candles, HashMap::new());
        let err = DailyProfile.calculate(&input).unwrap_err();
        assert_eq!(
            err,
            CalculationError::InsufficientData {
                required: 2,
                provided: 1
            }
        );
    }

    #[test]
    fn daily_profile_tightness() {
        let input = CalculationInput::new(make_intraday_candles(), HashMap::new());
        let result = DailyProfile.calculate(&input).unwrap();
        let tight = result.get("tightness").unwrap();
        // Day 1: high at idx 1, low at idx 0 -> tightness = 1
        assert!((tight[0] - 1.0).abs() < 1e-10);
        // Day 2: high at idx 0, low at idx 1 -> tightness = 1
        assert!((tight[1] - 1.0).abs() < 1e-10);
    }

    #[test]
    fn daily_profile_metadata_is_correct() {
        let meta = DailyProfile.metadata();
        assert_eq!(meta.id, "daily_profile");
        assert_eq!(meta.output_series.len(), 5);
        assert!(meta.dependencies.is_empty());
        assert_eq!(meta.price_fields, vec![PriceField::FullCandle]);
    }
}
