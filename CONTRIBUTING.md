# Contributing to market-calculations

Thank you for your interest in contributing! This document provides guidelines for contributing to this project.

## Getting Started

1. Fork the repository
2. Clone your fork: `git clone https://github.com/<your-username>/market-calculations.git`
3. Create a branch: `git checkout -b my-feature`
4. Make your changes
5. Push and open a pull request

## Development Setup

**Prerequisites:**
- Rust stable toolchain (install via [rustup](https://rustup.rs/))
- `rustfmt` and `clippy` components: `rustup component add rustfmt clippy`

**Build and test:**
```bash
cargo build
cargo test
cargo clippy -- -D warnings
cargo fmt --check
```

All four commands must pass before submitting a PR. CI enforces this.

## Adding a New Calculation

1. Create a new file in `src/calculations/` (e.g., `src/calculations/vwap.rs`)
2. Implement the `Calculation` trait:
   - `metadata()` — provide id, title, description, parameters, output series, and dependencies
   - `min_candles()` — return the minimum candle count for given params
   - `calculate()` — implement the calculation logic
3. Add the module to `src/calculations/mod.rs`
4. Register it in `CalculationRegistry::with_defaults()` in `src/registry.rs`
5. Write unit tests in the same file (see existing calculations for patterns)
6. Add integration tests in `tests/integration_tests.rs` if appropriate

**Checklist for new calculations:**
- [ ] Correct metadata with all fields populated (including `output_series` and `dependencies`)
- [ ] Parameter validation with clear error messages
- [ ] `f64::NAN` for warm-up period values
- [ ] Unit tests: known values, boundary (exact min candles, min-1), edge cases, parameter validation
- [ ] Integration test calling through the registry

## Code Style

- Run `cargo fmt` before committing
- Run `cargo clippy -- -D warnings` and fix all warnings
- Follow existing patterns in the codebase
- Keep functions focused and small
- Use descriptive variable names

## Pull Request Guidelines

- Keep PRs focused on a single change
- Write a clear description of what changed and why
- Reference any related issues
- Ensure all CI checks pass
- Be responsive to review feedback

## Reporting Issues

- Use the issue templates provided
- Include reproduction steps for bugs
- For feature requests, describe the use case and expected behavior

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
