pub mod calculations;
pub mod input;
pub mod metadata;
pub mod output;
pub mod pipeline;
pub mod registry;
pub mod traits;
pub mod types;

pub use input::CalculationInput;
pub use metadata::{CalculationMetadata, DependencyDef, OutputSeriesDef, ParamValue, SeriesType};
pub use output::{CalculationError, CalculationOutput, PRIMARY_SERIES};
pub use pipeline::Pipeline;
pub use registry::CalculationRegistry;
pub use traits::Calculation;
pub use types::Candle;
