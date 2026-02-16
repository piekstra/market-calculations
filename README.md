# market-calculations

High-performance Rust library for financial technical analysis calculations with a pluggable registry, JSON manifest discovery, and derivative calculation pipelines.

## Features

- **Uniform I/O contract** — Every calculation takes `CalculationInput` (candles + params + dependency outputs) and returns `CalculationOutput` (named series of f64 values).
- **Multi-series output** — Calculations can produce multiple named output series (e.g., daily profile produces range, high-before-low, tightness, gap%, intraday return).
- **Self-describing functions** — Each calculation declares metadata: id, title, description, parameters, output series definitions, data requirements, and dependencies.
- **Registry with dynamic discovery** — `CalculationRegistry` holds all calculations, supports lookup by id, and exports a JSON manifest for external systems.
- **Derivative calculations** — Calculations can depend on other calculations' outputs. The `Pipeline` automatically resolves and executes the dependency graph.
- **Extensible** — Implement the `Calculation` trait and register custom functions alongside built-ins.
- **Thread-safe** — All calculations are `Send + Sync` behind `Arc<dyn Calculation>` for high-throughput concurrent use.

## Built-in Calculations

| ID | Title | Type | Default Params |
|----|-------|------|---------------|
| `sma` | Simple Moving Average | Single-series | period=20 |
| `ema` | Exponential Moving Average | Single-series | period=20 |
| `rsi` | Relative Strength Index | Single-series | period=14 |
| `atr` | Average True Range | Single-series | period=14 |
| `obv` | On-Balance Volume | Single-series | N/A |
| `bollinger_bands` | Bollinger Bands | Derivative (SMA), multi-series (5) | period=20 |
| `macd` | MACD | Multi-series (3) | fast=12, slow=26, signal=9 |
| `stochastic` | Stochastic Oscillator | Multi-series (2) | k=14, d=3, smooth_k=3 |
| `vwap` | Volume-Weighted Average Price | Multi-series (3) | N/A |
| `daily_profile` | Daily Price Profile | Multi-series (5) | N/A |
| `range_trend` | Daily Range Trend | Derivative, multi-series (2) | window=5 |

## Usage

```rust
use market_calculations::*;
use std::collections::HashMap;

let registry = CalculationRegistry::with_defaults();

// Direct call for standalone calculations
let input = CalculationInput::new(
    vec![/* your OHLCV candles */],
    HashMap::from([("period".into(), ParamValue::Integer(200))]),
);
let sma = registry.get("sma").unwrap();
let result = sma.calculate(&input).unwrap();
let values = result.values().unwrap(); // primary series

// Pipeline for derivative calculations (auto-resolves dependencies)
let pipeline = Pipeline::new(&registry);
let result = pipeline.run(
    "range_trend",
    &candles,
    &HashMap::from([("window".into(), ParamValue::Integer(5))]),
).unwrap();
let trend = result.get("trend_score").unwrap();
let avg = result.get("avg_range").unwrap();

// Export manifest for external discovery
let manifest_json = registry.export_manifest().unwrap();
```

## Custom Calculations

```rust
use market_calculations::*;
use std::sync::Arc;

struct MyCalc;

impl Calculation for MyCalc {
    fn metadata(&self) -> CalculationMetadata { /* ... */ }
    fn min_candles(&self, _input: &CalculationInput) -> usize { 1 }
    fn calculate(&self, input: &CalculationInput)
        -> Result<CalculationOutput, CalculationError> {
        let values = input.candles.iter().map(|c| c.close).collect();
        Ok(CalculationOutput::single(values))
    }
}

let mut registry = CalculationRegistry::with_defaults();
registry.register(Arc::new(MyCalc));
```

## Multi-Series Output

Calculations that produce multiple series use `CalculationOutput::multi()`:

```rust
let mut series = HashMap::new();
series.insert("metric_a".to_string(), vec![1.0, 2.0, 3.0]);
series.insert("metric_b".to_string(), vec![4.0, 5.0, 6.0]);
Ok(CalculationOutput::multi(series))
```

Access named series with `result.get("metric_a")` or the primary series with `result.values()`.

## Derivative Calculations

Derivative calculations declare dependencies in their metadata and read pre-computed results from `input.dependencies`:

```rust
fn metadata(&self) -> CalculationMetadata {
    CalculationMetadata {
        dependencies: vec![DependencyDef {
            calculation_id: "daily_profile".into(),
            required_series: vec!["daily_range_pct".into()],
            default_params: HashMap::new(),
        }],
        // ...
    }
}

fn calculate(&self, input: &CalculationInput) -> Result<CalculationOutput, CalculationError> {
    let dep = input.dependencies.get("daily_profile").unwrap();
    let range_data = dep.get("daily_range_pct").unwrap();
    // ... compute based on dependency output
}
```

Use `Pipeline::run()` to automatically resolve and execute the dependency graph.
