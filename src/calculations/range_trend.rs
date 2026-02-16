use std::collections::HashMap;

use crate::input::CalculationInput;
use crate::metadata::*;
use crate::output::*;
use crate::traits::Calculation;

/// Rolling trend analysis of daily range data.
/// Depends on `daily_profile` output.
pub struct RangeTrend;

impl RangeTrend {
    fn get_window(input: &CalculationInput) -> Result<usize, CalculationError> {
        match input.params.get("window") {
            Some(ParamValue::Integer(w)) if *w >= 2 => Ok(*w as usize),
            Some(ParamValue::Integer(w)) => Err(CalculationError::InvalidParameter {
                name: "window".into(),
                reason: format!("window must be >= 2, got {}", w),
            }),
            Some(_) => Err(CalculationError::InvalidParameter {
                name: "window".into(),
                reason: "window must be an integer".into(),
            }),
            None => Ok(5),
        }
    }
}

impl Calculation for RangeTrend {
    fn metadata(&self) -> CalculationMetadata {
        CalculationMetadata {
            id: "range_trend".into(),
            title: "Daily Range Trend".into(),
            description: "Rolling trend analysis of daily range. Produces a trend score \
                (-1.0 to 1.0) indicating whether ranges are expanding or contracting, \
                and a rolling average range."
                .into(),
            parameters: vec![ParameterDef {
                name: "window".into(),
                description: "Number of days for the rolling trend window".into(),
                param_type: ParameterType::Integer,
                default_value: Some(ParamValue::Integer(5)),
                required: false,
            }],
            price_fields: vec![],
            min_candles_description:
                "Delegates to daily_profile; needs window + 1 trading days minimum".into(),
            output_series: vec![
                OutputSeriesDef {
                    name: "trend_score".into(),
                    description: "Normalized slope of daily range trend, clamped to [-1.0, 1.0]"
                        .into(),
                    series_type: SeriesType::Numeric,
                },
                OutputSeriesDef {
                    name: "avg_range".into(),
                    description: "Rolling average of daily_range_pct over the window".into(),
                    series_type: SeriesType::Percentage,
                },
            ],
            dependencies: vec![DependencyDef {
                calculation_id: "daily_profile".into(),
                required_series: vec!["daily_range_pct".into()],
                default_params: HashMap::new(),
            }],
        }
    }

    fn min_candles(&self, input: &CalculationInput) -> usize {
        Self::get_window(input).unwrap_or(5) + 1
    }

    fn calculate(&self, input: &CalculationInput) -> Result<CalculationOutput, CalculationError> {
        let window = Self::get_window(input)?;

        let daily_profile = input.dependencies.get("daily_profile").ok_or_else(|| {
            CalculationError::CalculationFailed("Missing required dependency: daily_profile".into())
        })?;

        let range_pct = daily_profile.get("daily_range_pct").ok_or_else(|| {
            CalculationError::CalculationFailed(
                "daily_profile missing daily_range_pct series".into(),
            )
        })?;

        let n = range_pct.len();
        if n < window {
            return Err(CalculationError::InsufficientData {
                required: window,
                provided: n,
            });
        }

        let mut trend_score = vec![f64::NAN; n];
        let mut avg_range = vec![f64::NAN; n];

        for i in (window - 1)..n {
            let slice = &range_pct[(i + 1 - window)..=i];

            // Rolling average
            let mean: f64 = slice.iter().sum::<f64>() / window as f64;
            avg_range[i] = mean;

            // Simple linear regression slope (normalized)
            let x_mean = (window as f64 - 1.0) / 2.0;
            let mut num = 0.0;
            let mut den = 0.0;
            for (j, val) in slice.iter().enumerate() {
                let x_diff = j as f64 - x_mean;
                num += x_diff * (val - mean);
                den += x_diff * x_diff;
            }
            let slope = if den != 0.0 { num / den } else { 0.0 };

            trend_score[i] = if mean != 0.0 {
                (slope / mean).clamp(-1.0, 1.0)
            } else {
                0.0
            };
        }

        let mut series = HashMap::new();
        series.insert("trend_score".to_string(), trend_score);
        series.insert("avg_range".to_string(), avg_range);

        Ok(CalculationOutput::multi(series))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn make_daily_profile_output(range_pcts: &[f64]) -> CalculationOutput {
        let mut series = HashMap::new();
        series.insert("daily_range_pct".to_string(), range_pcts.to_vec());
        // Add other series that daily_profile would produce (not needed by range_trend)
        series.insert("high_before_low".to_string(), vec![1.0; range_pcts.len()]);
        series.insert("tightness".to_string(), vec![1.0; range_pcts.len()]);
        series.insert("gap_pct".to_string(), vec![0.0; range_pcts.len()]);
        series.insert("open_to_close_pct".to_string(), vec![0.0; range_pcts.len()]);
        CalculationOutput::multi(series)
    }

    #[test]
    fn range_trend_expanding_ranges() {
        let deps = HashMap::from([(
            "daily_profile".to_string(),
            make_daily_profile_output(&[1.0, 2.0, 3.0, 4.0, 5.0]),
        )]);
        let input = CalculationInput::with_dependencies(vec![], HashMap::new(), deps);
        let result = RangeTrend.calculate(&input).unwrap();
        let scores = result.get("trend_score").unwrap();
        // With default window=5, only last value is defined
        assert!(scores[3].is_nan());
        assert!(
            scores[4] > 0.0,
            "Expanding ranges should have positive trend"
        );
    }

    #[test]
    fn range_trend_contracting_ranges() {
        let deps = HashMap::from([(
            "daily_profile".to_string(),
            make_daily_profile_output(&[5.0, 4.0, 3.0, 2.0, 1.0]),
        )]);
        let input = CalculationInput::with_dependencies(vec![], HashMap::new(), deps);
        let result = RangeTrend.calculate(&input).unwrap();
        let scores = result.get("trend_score").unwrap();
        assert!(
            scores[4] < 0.0,
            "Contracting ranges should have negative trend"
        );
    }

    #[test]
    fn range_trend_constant_ranges() {
        let deps = HashMap::from([(
            "daily_profile".to_string(),
            make_daily_profile_output(&[3.0, 3.0, 3.0, 3.0, 3.0]),
        )]);
        let input = CalculationInput::with_dependencies(vec![], HashMap::new(), deps);
        let result = RangeTrend.calculate(&input).unwrap();
        let scores = result.get("trend_score").unwrap();
        assert!(
            (scores[4] - 0.0).abs() < 1e-10,
            "Constant ranges should have zero trend"
        );
    }

    #[test]
    fn range_trend_avg_range() {
        let deps = HashMap::from([(
            "daily_profile".to_string(),
            make_daily_profile_output(&[1.0, 2.0, 3.0]),
        )]);
        let input = CalculationInput::with_dependencies(
            vec![],
            HashMap::from([("window".into(), ParamValue::Integer(3))]),
            deps,
        );
        let result = RangeTrend.calculate(&input).unwrap();
        let avg = result.get("avg_range").unwrap();
        assert!((avg[2] - 2.0).abs() < 1e-10); // (1+2+3)/3 = 2.0
    }

    #[test]
    fn range_trend_window_2() {
        let deps = HashMap::from([(
            "daily_profile".to_string(),
            make_daily_profile_output(&[2.0, 4.0, 6.0]),
        )]);
        let input = CalculationInput::with_dependencies(
            vec![],
            HashMap::from([("window".into(), ParamValue::Integer(2))]),
            deps,
        );
        let result = RangeTrend.calculate(&input).unwrap();
        let scores = result.get("trend_score").unwrap();
        assert!(scores[0].is_nan());
        assert!(scores[1] > 0.0); // 2->4 expanding
        assert!(scores[2] > 0.0); // 4->6 expanding
    }

    #[test]
    fn range_trend_insufficient_data() {
        let deps = HashMap::from([(
            "daily_profile".to_string(),
            make_daily_profile_output(&[1.0, 2.0]),
        )]);
        let input = CalculationInput::with_dependencies(vec![], HashMap::new(), deps);
        // Default window=5, only 2 data points
        assert_eq!(
            RangeTrend.calculate(&input).unwrap_err(),
            CalculationError::InsufficientData {
                required: 5,
                provided: 2
            }
        );
    }

    #[test]
    fn range_trend_missing_dependency() {
        let input = CalculationInput::new(vec![], HashMap::new());
        assert!(matches!(
            RangeTrend.calculate(&input),
            Err(CalculationError::CalculationFailed(_))
        ));
    }

    #[test]
    fn range_trend_invalid_window() {
        let deps = HashMap::from([(
            "daily_profile".to_string(),
            make_daily_profile_output(&[1.0, 2.0, 3.0]),
        )]);
        let input = CalculationInput::with_dependencies(
            vec![],
            HashMap::from([("window".into(), ParamValue::Integer(1))]),
            deps,
        );
        assert!(matches!(
            RangeTrend.calculate(&input),
            Err(CalculationError::InvalidParameter { .. })
        ));
    }

    #[test]
    fn range_trend_metadata_is_correct() {
        let meta = RangeTrend.metadata();
        assert_eq!(meta.id, "range_trend");
        assert_eq!(meta.output_series.len(), 2);
        assert_eq!(meta.dependencies.len(), 1);
        assert_eq!(meta.dependencies[0].calculation_id, "daily_profile");
        assert!(meta.price_fields.is_empty());
    }

    #[test]
    fn range_trend_score_clamped() {
        // Extreme data to test clamping
        let deps = HashMap::from([(
            "daily_profile".to_string(),
            make_daily_profile_output(&[0.001, 100.0]),
        )]);
        let input = CalculationInput::with_dependencies(
            vec![],
            HashMap::from([("window".into(), ParamValue::Integer(2))]),
            deps,
        );
        let result = RangeTrend.calculate(&input).unwrap();
        let scores = result.get("trend_score").unwrap();
        assert!(scores[1] <= 1.0);
        assert!(scores[1] >= -1.0);
    }
}
