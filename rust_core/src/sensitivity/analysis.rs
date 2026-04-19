use crate::dataframe;

use super::SensitivityResult;

mod common;
mod full;
mod selected;

pub use full::{compute_sensitivity_all, compute_sensitivity_without_mdi};
pub use selected::compute_sensitivity_selected;

pub fn compute_sensitivity() -> Option<SensitivityResult> {
    dataframe::with_active_df(compute_sensitivity_all)
}
