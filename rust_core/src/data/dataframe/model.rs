use super::buffers::{build_positions, build_positions3d};
use super::types::{DataFrameInfo, GpuBufferData, TrialRow};

/// Documentation.
/// Documentation.
#[derive(Clone, Debug)]
pub struct DataFrame {
    row_count: usize,
    /// Documentation.
    trial_ids: Vec<u32>,
    /// Documentation.
    numeric_cols: Vec<(String, Vec<f64>)>,
    /// Documentation.
    string_cols: Vec<(String, Vec<String>)>,
    /// Documentation.
    param_col_names: Vec<String>,
    objective_col_names: Vec<String>,
    user_attr_numeric_col_names: Vec<String>,
    user_attr_string_col_names: Vec<String>,
    constraint_col_names: Vec<String>,
    /// derived columns: is_feasible, constraint_sum 🟢
    derived_col_names: Vec<String>,
}

impl DataFrame {
    /// Documentation.
    pub fn empty() -> Self {
        DataFrame {
            row_count: 0,
            trial_ids: vec![],
            numeric_cols: vec![],
            string_cols: vec![],
            param_col_names: vec![],
            objective_col_names: vec![],
            user_attr_numeric_col_names: vec![],
            user_attr_string_col_names: vec![],
            constraint_col_names: vec![],
            derived_col_names: vec![],
        }
    }

    /// Documentation.
    ///
    /// Documentation.
    /// Documentation.
    /// Documentation.
    /// Documentation.
    /// Documentation.
    /// Documentation.
    /// Documentation.
    /// Documentation.
    pub fn from_trials(
        trial_rows: &[TrialRow],
        param_names: &[String],
        objective_names: &[String],
        user_attr_numeric_names: &[String],
        user_attr_string_names: &[String],
        max_constraints: usize,
    ) -> Self {
        let n = trial_rows.len();
        if n == 0 {
            return DataFrame::empty();
        }

        let trial_ids: Vec<u32> = trial_rows.iter().map(|r| r.trial_id).collect();

        let mut numeric_cols: Vec<(String, Vec<f64>)> = Vec::new();
        let mut string_cols: Vec<(String, Vec<String>)> = Vec::new();
        let mut param_col_names = Vec::new();
        let mut objective_col_names = Vec::new();
        let mut user_attr_numeric_col_names = Vec::new();
        let mut user_attr_string_col_names = Vec::new();
        let mut constraint_col_names = Vec::new();
        let mut derived_col_names = Vec::new();

        for name in param_names {
            let has_label = trial_rows
                .iter()
                .any(|r| r.param_category_label.contains_key(name));
            if has_label {
                let vals: Vec<String> = trial_rows
                    .iter()
                    .map(|r| {
                        r.param_category_label
                            .get(name)
                            .cloned()
                            .unwrap_or_default()
                    })
                    .collect();
                string_cols.push((name.clone(), vals));
            } else {
                let vals: Vec<f64> = trial_rows
                    .iter()
                    .map(|r| *r.param_display.get(name).unwrap_or(&0.0))
                    .collect();
                numeric_cols.push((name.clone(), vals));
            }
            param_col_names.push(name.clone());
        }

        for (i, name) in objective_names.iter().enumerate() {
            let vals: Vec<f64> = trial_rows
                .iter()
                .map(|r| r.objective_values.get(i).copied().unwrap_or(f64::NAN))
                .collect();
            numeric_cols.push((name.clone(), vals));
            objective_col_names.push(name.clone());
        }

        for name in user_attr_numeric_names {
            let vals: Vec<f64> = trial_rows
                .iter()
                .map(|r| *r.user_attrs_numeric.get(name).unwrap_or(&f64::NAN))
                .collect();
            numeric_cols.push((name.clone(), vals));
            user_attr_numeric_col_names.push(name.clone());
        }

        for name in user_attr_string_names {
            let vals: Vec<String> = trial_rows
                .iter()
                .map(|r| r.user_attrs_string.get(name).cloned().unwrap_or_default())
                .collect();
            string_cols.push((name.clone(), vals));
            user_attr_string_col_names.push(name.clone());
        }

        if max_constraints > 0 {
            for ci in 0..max_constraints {
                let col_name = format!("c{}", ci + 1);
                let vals: Vec<f64> = trial_rows
                    .iter()
                    .map(|r| r.constraint_values.get(ci).copied().unwrap_or(0.0))
                    .collect();
                numeric_cols.push((col_name.clone(), vals));
                constraint_col_names.push(col_name);
            }

            let is_feasible_vals: Vec<f64> = trial_rows
                .iter()
                .map(|r| {
                    if r.constraint_values.iter().all(|&c| c <= 0.0) {
                        1.0
                    } else {
                        0.0
                    }
                })
                .collect();
            numeric_cols.push(("is_feasible".to_string(), is_feasible_vals));
            derived_col_names.push("is_feasible".to_string());

            let sum_vals: Vec<f64> = trial_rows
                .iter()
                .map(|r| r.constraint_values.iter().sum())
                .collect();
            numeric_cols.push(("constraint_sum".to_string(), sum_vals));
            derived_col_names.push("constraint_sum".to_string());
        }

        DataFrame {
            row_count: n,
            trial_ids,
            numeric_cols,
            string_cols,
            param_col_names,
            objective_col_names,
            user_attr_numeric_col_names,
            user_attr_string_col_names,
            constraint_col_names,
            derived_col_names,
        }
    }

    /// Documentation.
    pub fn get_trial_id(&self, row: usize) -> Option<u32> {
        self.trial_ids.get(row).copied()
    }

    /// Documentation.
    pub fn get_trial_number(&self, row: usize) -> Option<u32> {
        if row < self.row_count {
            Some(row as u32)
        } else {
            None
        }
    }

    /// Documentation.
    pub fn param_col_names(&self) -> &[String] {
        &self.param_col_names
    }

    pub fn objective_col_names(&self) -> &[String] {
        &self.objective_col_names
    }

    pub fn user_attr_numeric_col_names(&self) -> &[String] {
        &self.user_attr_numeric_col_names
    }

    pub fn user_attr_string_col_names(&self) -> &[String] {
        &self.user_attr_string_col_names
    }

    pub fn constraint_col_names(&self) -> &[String] {
        &self.constraint_col_names
    }

    /// Documentation.
    pub fn row_count(&self) -> usize {
        self.row_count
    }

    /// Documentation.
    pub fn column_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.numeric_cols.iter().map(|(n, _)| n.clone()).collect();
        names.extend(self.string_cols.iter().map(|(n, _)| n.clone()));
        names
    }

    /// Documentation.
    pub fn get_numeric_column(&self, name: &str) -> Option<&[f64]> {
        self.numeric_cols
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.as_slice())
    }

    /// Documentation.
    pub fn get_string_column(&self, name: &str) -> Option<&[String]> {
        self.string_cols
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.as_slice())
    }

    /// Documentation.
    pub fn info(&self) -> DataFrameInfo {
        let mut all_user_attr: Vec<String> = self.user_attr_numeric_col_names.clone();
        all_user_attr.extend(self.user_attr_string_col_names.iter().cloned());

        DataFrameInfo {
            row_count: self.row_count,
            column_names: self.column_names(),
            param_columns: self.param_col_names.clone(),
            objective_columns: self.objective_col_names.clone(),
            user_attr_columns: all_user_attr,
            constraint_columns: self.constraint_col_names.clone(),
            derived_columns: self.derived_col_names.clone(),
        }
    }

    /// Documentation.
    pub fn gpu_buffers(&self) -> GpuBufferData {
        let n = self.row_count;
        let positions = build_positions(self, n);
        let positions3d = build_positions3d(self, n);
        let sizes = vec![1.0f32; n];
        GpuBufferData {
            positions,
            positions3d,
            sizes,
            trial_count: n,
        }
    }

    /// Return a new `DataFrame` containing only rows where `is_feasible > 0.5`.
    ///
    /// If the `is_feasible` column does not exist (unconstrained study), all
    /// rows are retained unchanged.
    pub fn filter_feasible(&self) -> DataFrame {
        let feas = self.feasibility();
        let mask: Vec<bool> = (0..self.row_count).map(|i| feas.is_feasible(i)).collect();
        self.filter_rows(&mask)
    }

    /// Return a new `DataFrame` keeping only the rows for which `mask[i]` is `true`.
    fn filter_rows(&self, mask: &[bool]) -> DataFrame {
        let trial_ids: Vec<u32> = self
            .trial_ids
            .iter()
            .enumerate()
            .filter_map(|(i, &id)| {
                if mask.get(i).copied().unwrap_or(false) {
                    Some(id)
                } else {
                    None
                }
            })
            .collect();

        let numeric_cols: Vec<(String, Vec<f64>)> = self
            .numeric_cols
            .iter()
            .map(|(name, vals)| {
                let filtered: Vec<f64> = vals
                    .iter()
                    .enumerate()
                    .filter_map(|(i, &v)| {
                        if mask.get(i).copied().unwrap_or(false) {
                            Some(v)
                        } else {
                            None
                        }
                    })
                    .collect();
                (name.clone(), filtered)
            })
            .collect();

        let string_cols: Vec<(String, Vec<String>)> = self
            .string_cols
            .iter()
            .map(|(name, vals)| {
                let filtered: Vec<String> = vals
                    .iter()
                    .enumerate()
                    .filter_map(|(i, v)| {
                        if mask.get(i).copied().unwrap_or(false) {
                            Some(v.clone())
                        } else {
                            None
                        }
                    })
                    .collect();
                (name.clone(), filtered)
            })
            .collect();

        DataFrame {
            row_count: trial_ids.len(),
            trial_ids,
            numeric_cols,
            string_cols,
            param_col_names: self.param_col_names.clone(),
            objective_col_names: self.objective_col_names.clone(),
            user_attr_numeric_col_names: self.user_attr_numeric_col_names.clone(),
            user_attr_string_col_names: self.user_attr_string_col_names.clone(),
            constraint_col_names: self.constraint_col_names.clone(),
            derived_col_names: self.derived_col_names.clone(),
        }
    }
}
