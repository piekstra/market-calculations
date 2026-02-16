# Architecture

## Purpose

`market-calculations` is a **pure, stateless computation library** for financial technical analysis. It takes in OHLCV candle data and parameters, and returns calculated results. That's it.

## Core Principles

1. **Stateless** — No calculation retains state between invocations. Same inputs always produce the same outputs. There is no concept of "updating" a previous result.

2. **No I/O** — This library does not fetch market data, read from databases, make network requests, or access the filesystem. Candle data is provided by the caller.

3. **No caching** — Result caching, memoization, and incremental computation are the responsibility of the consuming application, not this library. This is intentional — the orchestration layer (e.g., TIRDS) owns cache invalidation, TTLs, and storage.

4. **Uniform contract** — Every calculation implements the same `Calculation` trait with the same input/output types. A system calling one calculation can call any calculation without bespoke handling.

5. **Self-describing** — Every calculation declares its own metadata: what it computes, what parameters it accepts, what output series it produces, and what other calculations it depends on. This enables dynamic discovery via JSON manifest export.

6. **Composable** — Calculations can declare dependencies on other calculations. The `Pipeline` resolves the dependency graph and passes intermediate results automatically. But each individual calculation remains a pure function of its inputs.

7. **Thread-safe** — All calculations are `Send + Sync` and stored behind `Arc<dyn Calculation>`. The library is designed for high-throughput concurrent use (millions of invocations/day).

## Boundaries

### This library IS responsible for:
- Defining the `Calculation` trait and I/O types (`CalculationInput`, `CalculationOutput`, `Candle`)
- Implementing built-in technical indicators (SMA, EMA, RSI, ATR, MACD, etc.)
- Providing a registry for dynamic lookup and manifest export
- Resolving calculation dependency graphs via `Pipeline`
- Parameter validation and error reporting

### This library is NOT responsible for:
- Fetching or storing market data
- Caching calculation results
- Incremental/streaming computation (appending new candles to prior results)
- Scheduling or periodic recalculation
- Knowing about symbols, timeframes, exchanges, or trading sessions
- Authentication, authorization, or network communication

## Where caching and orchestration live

The `tirds-loader` component in [piekstra/tirds](https://github.com/piekstra/tirds) is the orchestration layer that:
- Fetches candle data from [piekstra/market-data](https://github.com/piekstra/market-data)
- Invokes calculations from this library via `CalculationRegistry` and `Pipeline`
- Caches results in a two-tier cache (in-memory + SQLite)
- Manages refresh cycles and cache expiration

If you're considering adding statefulness, caching, or data fetching to this library — stop and consider whether that concern belongs in the orchestration layer instead.

## Key types

```
CalculationInput       Candles + params + dependency outputs (provided by caller or Pipeline)
CalculationOutput      Named series map: HashMap<String, Vec<f64>>
CalculationMetadata    Self-description: id, title, params, output series, dependencies
Calculation trait      metadata() + calculate() + min_candles() + validate_input()
CalculationRegistry    Collection of calculations with lookup and manifest export
Pipeline               Dependency-resolving executor that runs a calculation and its prerequisites
```
