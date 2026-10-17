//! Caller-owned Study authority. No storage, analysis, or active selection lives here.
mod manager;
mod model;

pub use manager::*;
pub use model::*;
