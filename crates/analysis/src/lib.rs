//! Analysis on caller-owned immutable Study inputs. No storage or active selection.
pub mod clustering;
pub mod contour;
pub mod convergence;
pub mod data;
pub mod export;
pub(crate) mod gaussian_process;
pub(crate) mod lgbm;
pub(crate) mod lgbm_sys;
pub mod math;
pub mod mcdm;
pub mod multi_objective;
pub mod optimization;
pub mod pdp;
pub mod preparation;
pub mod projection;
pub mod report;
pub mod sensitivity;
pub mod session;
pub mod statistics;
pub mod surrogate_opt;
pub mod training;
pub use data::{dataframe, extras, filter};
pub use mcdm::{entropy, promethee, topsis, vikor};
pub use multi_objective::{indicators, pareto};
pub use report::{build_study_report, render_markdown, ReportLang, ReportOptions, ReportSource};
#[cfg(test)]
mod native_tests;
