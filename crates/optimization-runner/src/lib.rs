//! Real-evaluation optimization with distinct generic-process and Grasshopper paths.
//!
//! [`runner`] drives generic evaluators, including [`process::ProcessEvaluator`].
//! [`gh`] owns GHX parsing, Rhino.Compute lifecycle and its method-specific loops.
//! Both record through `tdashboard-io` and reuse numerical algorithms, RNG and
//! `FitProgress` from `tdashboard-analysis`; neither calls legacy core orchestration.
pub mod gh;
pub mod process;
pub mod runner;

#[cfg(test)]
mod test_support;
