use crate::input::CalculationInput;
use crate::metadata::CalculationMetadata;
use crate::output::{CalculationError, CalculationOutput};

/// The core trait that every calculation must implement.
///
/// Object-safe (`Send + Sync`) so it can be stored as `Arc<dyn Calculation>`
/// and shared across threads for high-throughput use.
pub trait Calculation: Send + Sync {
    /// Returns metadata describing this calculation.
    fn metadata(&self) -> CalculationMetadata;

    /// Validates input and runs the calculation.
    fn calculate(&self, input: &CalculationInput) -> Result<CalculationOutput, CalculationError>;

    /// Computes the minimum number of candles required for the given params.
    fn min_candles(&self, input: &CalculationInput) -> usize;

    /// Validates that input has enough candles.
    fn validate_input(&self, input: &CalculationInput) -> Result<(), CalculationError> {
        let required = self.min_candles(input);
        let provided = input.candles.len();
        if provided < required {
            Err(CalculationError::InsufficientData { required, provided })
        } else {
            Ok(())
        }
    }
}
