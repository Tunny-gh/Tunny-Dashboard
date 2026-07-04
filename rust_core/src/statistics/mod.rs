pub mod boxplot;
pub mod correlation;
pub mod distribution_fit;
pub mod histogram;

pub use boxplot::{compute_boxplot, quantile, BoxPlotStats};
pub use correlation::{compute_correlation_matrix, CorrelationMatrix, CorrelationMethod};
pub use distribution_fit::{fit_all, fit_distribution, FitDistribution, FittedDistribution};
pub use histogram::{compute_histogram, BinRule, Histogram};
