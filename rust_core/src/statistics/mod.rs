pub mod boxplot;
pub mod correlation;
pub mod histogram;

pub use boxplot::{compute_boxplot, quantile, BoxPlotStats};
pub use correlation::{compute_correlation_matrix, CorrelationMatrix, CorrelationMethod};
pub use histogram::{compute_histogram, BinRule, Histogram};
