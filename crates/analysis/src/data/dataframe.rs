//! Derived COMPLETE-only tables. The Study input remains the trial authority.
mod feasibility;
mod model;
mod types;
pub use feasibility::{Feasibility, FeasibilityState};
pub use model::DataFrame;
pub use types::{DistributionMetadata, TrialRow};
