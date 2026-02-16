# market-calculations

High-performance Rust library for financial technical analysis calculations with a pluggable registry and JSON manifest discovery.

## Features

- **Uniform I/O contract** — Every calculation takes `CalculationInput` (candles + params) and returns `CalculationOutput` (aligned f64 values). No bespoke handling per function.
- **Self-describing functions** — Each calculation declares its metadata: id, title, description, parameters, data requirements.
- **Registry with dynamic discovery** — `CalculationRegistry` holds all calculations, supports lookup by id, and exports a JSON manifest for external systems.
- **Extensible** — Implement the `Calculation` trait and register custom functions alongside built-ins.
- **Thread-safe** — All calculations are `Send + Sync` behind `Arc<dyn Calculation>` for high-throughput concurrent use.

## Built-in Calculations

| ID | Title | Default Period |
|----|-------|---------------|
| `sma` | Simple Moving Average | 20 |
| `ema` | Exponential Moving Average | 20 |
| `rsi` | Relative Strength Index | 14 |

## Usage

```rust
use market_calculations::*;
use std::collections::HashMap;

let registry = CalculationRegistry::with_defaults();

let input = CalculationInput {
    candles: vec![/* your OHLCV candles */],
    params: HashMap::from([("period".into(), ParamValue::Integer(200))]),
};

// Call any calculation by id
let sma = registry.get("sma").unwrap();
let result = sma.calculate(&input).unwrap();

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
        -> Result<CalculationOutput, CalculationError> { /* ... */ }
}

let mut registry = CalculationRegistry::with_defaults();
registry.register(Arc::new(MyCalc));
```

## JSON Manifest

`registry.export_manifest()` produces a JSON array describing all registered calculations:

```json
[
  {
    "id": "sma",
    "title": "Simple Moving Average",
    "description": "Arithmetic mean of closing prices over the specified period.",
    "parameters": [
      {
        "name": "period",
        "description": "Number of candles to average",
        "param_type": "integer",
        "default_value": 20,
        "required": false
      }
    ],
    "price_fields": ["close"],
    "min_candles_description": "Equal to the period parameter"
  }
]
```
