pub mod clustering;
pub(crate) mod core;
pub mod data;
pub mod io;
pub(crate) mod lgbm_sys;
pub mod mcdm;
pub mod multi_objective;
pub mod pdp;
pub mod sampling;
pub mod sensitivity;

pub use data::{dataframe, filter};
pub use io::journal::{live_update, parser as journal_parser};
pub use io::{export, journal};
pub use mcdm::ahp;
pub use mcdm::entropy;
pub use mcdm::promethee;
pub use mcdm::topsis;
pub use mcdm::vikor;
pub use multi_objective::pareto;

#[cfg(test)]
mod tests {
    mod integration;

    #[test]
    fn lib_compiles() {
        assert!(true);
    }

    #[test]
    fn wasm_get_trials_no_active_study_returns_empty() {
        let result = crate::dataframe::with_active_df(|_df| 42usize);
        assert!(
            result.is_none(),
            "translated Study translated None translated"
        );
    }

    #[test]
    fn wasm_get_trials_with_dataframe() {
        use crate::dataframe::{
            select_study, store_dataframes, with_active_df, DataFrame, TrialRow,
        };
        use std::collections::HashMap;

        let rows = vec![
            TrialRow {
                trial_id: 0,
                param_display: [("x".to_string(), 1.5)].iter().cloned().collect(),
                param_category_label: HashMap::new(),
                objective_values: vec![10.0],
                user_attrs_numeric: HashMap::new(),
                user_attrs_string: HashMap::new(),
                constraint_values: vec![],
            },
            TrialRow {
                trial_id: 1,
                param_display: [("x".to_string(), 2.5)].iter().cloned().collect(),
                param_category_label: HashMap::new(),
                objective_values: vec![5.0],
                user_attrs_numeric: HashMap::new(),
                user_attrs_string: HashMap::new(),
                constraint_values: vec![],
            },
        ];

        let df = DataFrame::from_trials(
            &rows,
            &["x".to_string()],
            &["obj0".to_string()],
            &[],
            &[],
            0,
        );

        store_dataframes(vec![df]);
        select_study(0).unwrap();

        let param_count = with_active_df(|df| df.param_col_names().len()).unwrap_or(0);
        assert_eq!(param_count, 1);

        let row_count = with_active_df(|df| df.row_count()).unwrap_or(0);
        assert_eq!(row_count, 2);

        let x_values: Vec<f64> = with_active_df(|df| {
            df.get_numeric_column("x")
                .map(|col| col.to_vec())
                .unwrap_or_default()
        })
        .unwrap_or_default();
        assert_eq!(x_values, vec![1.5, 2.5]);

        let obj_values: Vec<f64> = with_active_df(|df| {
            df.get_numeric_column("obj0")
                .map(|col| col.to_vec())
                .unwrap_or_default()
        })
        .unwrap_or_default();
        assert_eq!(obj_values, vec![10.0, 5.0]);
    }
}
