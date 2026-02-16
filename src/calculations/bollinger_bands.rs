use std::collections::HashMap;

use crate::input::CalculationInput;
use crate::metadata::*;
use crate::output::*;
use crate::traits::Calculation;
use crate::types::PriceField;

/// Bollinger Bands: volatility bands above and below a simple moving average.
pub struct BollingerBands;

impl BollingerBands {
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

    fn get_std_dev(input: &CalculationInput) -> Result<f64, CalculationError> {
        match input.params.get("std_dev") {
            Some(ParamValue::Float(s)) if *s > 0.0 => Ok(*s),
            Some(ParamValue::Float(s)) => Err(CalculationError::InvalidParameter {
                name: "std_dev".into(),
                reason: format!("std_dev must be positive, got {}", s),
            }),
            Some(ParamValue::Integer(s)) if *s > 0 => Ok(*s as f64),
            Some(_) => Err(CalculationError::InvalidParameter {
                name: "std_dev".into(),
                reason: "std_dev must be a number".into(),
            }),
            None => Ok(2.0),
        }
    }
}

impl Calculation for BollingerBands {
    fn metadata(&self) -> CalculationMetadata {
        CalculationMetadata {
            id: "bollinger_bands".into(),
            title: "Bollinger Bands".into(),
            description: "Volatility bands placed above and below a simple moving average. \
                Upper = SMA + (std_dev × σ), Lower = SMA - (std_dev × σ). \
                Bandwidth and %B measure band width and price position."
                .into(),
            parameters: vec![
                ParameterDef {
                    name: "period".into(),
                    description: "SMA period for middle band".into(),
                    param_type: ParameterType::Integer,
                    default_value: Some(ParamValue::Integer(20)),
                    required: false,
                },
                ParameterDef {
                    name: "std_dev".into(),
                    description: "Standard deviation multiplier for bands".into(),
                    param_type: ParameterType::Float,
                    default_value: Some(ParamValue::Float(2.0)),
                    required: false,
                },
            ],
            price_fields: vec![PriceField::Close],
            min_candles_description: "period".into(),
            output_series: vec![
                OutputSeriesDef {
                    name: "upper".into(),
                    description: "Upper band (SMA + std_dev × σ)".into(),
                    series_type: SeriesType::Numeric,
                },
                OutputSeriesDef {
                    name: "middle".into(),
                    description: "Middle band (SMA)".into(),
                    series_type: SeriesType::Numeric,
                },
                OutputSeriesDef {
                    name: "lower".into(),
                    description: "Lower band (SMA - std_dev × σ)".into(),
                    series_type: SeriesType::Numeric,
                },
                OutputSeriesDef {
                    name: "bandwidth".into(),
                    description: "Band width: (upper - lower) / middle × 100".into(),
                    series_type: SeriesType::Percentage,
                },
                OutputSeriesDef {
                    name: "percent_b".into(),
                    description: "%B: (price - lower) / (upper - lower)".into(),
                    series_type: SeriesType::Numeric,
                },
            ],
            dependencies: vec![DependencyDef {
                calculation_id: "sma".into(),
                required_series: vec!["value".into()],
                default_params: HashMap::new(),
            }],
        }
    }

    fn min_candles(&self, input: &CalculationInput) -> usize {
        Self::get_period(input).unwrap_or(20)
    }

    fn calculate(&self, input: &CalculationInput) -> Result<CalculationOutput, CalculationError> {
        self.validate_input(input)?;
        let period = Self::get_period(input)?;
        let std_multiplier = Self::get_std_dev(input)?;
        let closes: Vec<f64> = input.candles.iter().map(|c| c.close).collect();
        let n = closes.len();

        // Get SMA from dependencies
        let sma_output = input.dependencies.get("sma").ok_or_else(|| {
            CalculationError::CalculationFailed("Missing required dependency: sma".into())
        })?;
        let sma_values = sma_output.get("value").ok_or_else(|| {
            CalculationError::CalculationFailed("SMA output missing 'value' series".into())
        })?;

        let mut upper = vec![f64::NAN; n];
        let mut middle = vec![f64::NAN; n];
        let mut lower = vec![f64::NAN; n];
        let mut bandwidth = vec![f64::NAN; n];
        let mut percent_b = vec![f64::NAN; n];

        for i in (period - 1)..n {
            let sma = sma_values[i];
            if sma.is_nan() {
                continue;
            }

            // Calculate rolling standard deviation
            let window = &closes[(i + 1 - period)..=i];
            let variance: f64 =
                window.iter().map(|x| (x - sma).powi(2)).sum::<f64>() / period as f64;
            let std_dev = variance.sqrt();

            middle[i] = sma;
            upper[i] = sma + std_multiplier * std_dev;
            lower[i] = sma - std_multiplier * std_dev;

            if sma != 0.0 {
                bandwidth[i] = (upper[i] - lower[i]) / sma * 100.0;
            }

            let band_width = upper[i] - lower[i];
            if band_width.abs() > f64::EPSILON {
                percent_b[i] = (closes[i] - lower[i]) / band_width;
            } else {
                percent_b[i] = 0.5; // Flat bands
            }
        }

        let mut series = HashMap::new();
        series.insert("upper".to_string(), upper);
        series.insert("middle".to_string(), middle);
        series.insert("lower".to_string(), lower);
        series.insert("bandwidth".to_string(), bandwidth);
        series.insert("percent_b".to_string(), percent_b);

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

    fn sma_output(closes: &[f64], period: usize) -> CalculationOutput {
        let n = closes.len();
        let mut values = vec![f64::NAN; n];
        for i in (period - 1)..n {
            let sum: f64 = closes[(i + 1 - period)..=i].iter().sum();
            values[i] = sum / period as f64;
        }
        CalculationOutput::single(values)
    }

    #[test]
    fn bb_constant_prices_zero_bandwidth() {
        let closes = vec![100.0; 25];
        let candles = make_candles(&closes);
        let sma = sma_output(&closes, 20);

        let input = CalculationInput::with_dependencies(
            candles,
            HashMap::from([
                ("period".into(), ParamValue::Integer(20)),
                ("std_dev".into(), ParamValue::Float(2.0)),
            ]),
            HashMap::from([("sma".into(), sma)]),
        );

        let result = BollingerBands.calculate(&input).unwrap();
        let bw = result.get("bandwidth").unwrap();
        let pb = result.get("percent_b").unwrap();

        // Constant prices → zero std dev → zero bandwidth
        assert!((bw[19] - 0.0).abs() < 1e-10);
        // %B at midpoint when bands are flat
        assert!((pb[19] - 0.5).abs() < 1e-10);
    }

    #[test]
    fn bb_bands_symmetric_around_sma() {
        let closes: Vec<f64> = (0..25)
            .map(|i| 100.0 + (i as f64 * 0.3).sin() * 5.0)
            .collect();
        let candles = make_candles(&closes);
        let sma = sma_output(&closes, 20);

        let input = CalculationInput::with_dependencies(
            candles,
            HashMap::from([
                ("period".into(), ParamValue::Integer(20)),
                ("std_dev".into(), ParamValue::Float(2.0)),
            ]),
            HashMap::from([("sma".into(), sma)]),
        );

        let result = BollingerBands.calculate(&input).unwrap();
        let upper = result.get("upper").unwrap();
        let middle = result.get("middle").unwrap();
        let lower = result.get("lower").unwrap();

        for i in 19..25 {
            let upper_dist = upper[i] - middle[i];
            let lower_dist = middle[i] - lower[i];
            assert!(
                (upper_dist - lower_dist).abs() < 1e-10,
                "Bands not symmetric at index {}",
                i
            );
        }
    }

    #[test]
    fn bb_percent_b_at_upper_is_1() {
        // Price exactly at upper band: need price = SMA + 2*stddev
        // Use volatile data so bands are wide
        let closes = vec![
            95.0, 105.0, 95.0, 105.0, 95.0, 105.0, 95.0, 105.0, 95.0, 105.0, 95.0, 105.0, 95.0,
            105.0, 95.0, 105.0, 95.0, 105.0, 95.0, 105.0,
        ];
        let candles = make_candles(&closes);
        let sma = sma_output(&closes, 20);

        let input = CalculationInput::with_dependencies(
            candles,
            HashMap::from([
                ("period".into(), ParamValue::Integer(20)),
                ("std_dev".into(), ParamValue::Float(2.0)),
            ]),
            HashMap::from([("sma".into(), sma)]),
        );

        let result = BollingerBands.calculate(&input).unwrap();
        let pb = result.get("percent_b").unwrap();
        let upper = result.get("upper").unwrap();

        // Last close is 105, if it equals the upper band, %B = 1.0
        // Otherwise %B should be between 0 and 1 for normal values
        assert!(!pb[19].is_nan());
        // With alternating 95/105, SMA=100, stddev=5, upper=110, lower=90
        // %B = (105 - 90) / (110 - 90) = 15/20 = 0.75
        assert!((pb[19] - 0.75).abs() < 1e-10, "Got %B = {}", pb[19]);
        assert!((upper[19] - 110.0).abs() < 1e-10);
    }

    #[test]
    fn bb_warmup_is_nan() {
        let closes: Vec<f64> = (0..25).map(|i| 100.0 + i as f64).collect();
        let candles = make_candles(&closes);
        let sma = sma_output(&closes, 20);

        let input = CalculationInput::with_dependencies(
            candles,
            HashMap::from([
                ("period".into(), ParamValue::Integer(20)),
                ("std_dev".into(), ParamValue::Float(2.0)),
            ]),
            HashMap::from([("sma".into(), sma)]),
        );

        let result = BollingerBands.calculate(&input).unwrap();
        let middle = result.get("middle").unwrap();
        assert!(middle[18].is_nan());
        assert!(!middle[19].is_nan());
    }

    #[test]
    fn bb_missing_sma_dependency() {
        let closes = vec![100.0; 25];
        let candles = make_candles(&closes);

        let input = CalculationInput::new(
            candles,
            HashMap::from([
                ("period".into(), ParamValue::Integer(20)),
                ("std_dev".into(), ParamValue::Float(2.0)),
            ]),
        );

        assert!(matches!(
            BollingerBands.calculate(&input),
            Err(CalculationError::CalculationFailed(_))
        ));
    }

    #[test]
    fn bb_insufficient_data() {
        let closes = vec![100.0; 15];
        let candles = make_candles(&closes);
        let sma = sma_output(&closes, 20);

        let input = CalculationInput::with_dependencies(
            candles,
            HashMap::from([
                ("period".into(), ParamValue::Integer(20)),
                ("std_dev".into(), ParamValue::Float(2.0)),
            ]),
            HashMap::from([("sma".into(), sma)]),
        );

        assert_eq!(
            BollingerBands.calculate(&input).unwrap_err(),
            CalculationError::InsufficientData {
                required: 20,
                provided: 15,
            }
        );
    }

    #[test]
    fn bb_metadata() {
        let meta = BollingerBands.metadata();
        assert_eq!(meta.id, "bollinger_bands");
        assert_eq!(meta.parameters.len(), 2);
        assert_eq!(meta.dependencies.len(), 1);
        assert_eq!(meta.dependencies[0].calculation_id, "sma");
        assert_eq!(meta.output_series.len(), 5);
    }
}
