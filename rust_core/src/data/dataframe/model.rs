use std::collections::{HashMap, VecDeque};

use super::types::TrialRow;

/// A column-oriented Trial table (a lightweight DataFrame that looks up
/// numeric and string columns by name). Built by the journal / RDB parsers
/// and shared by the UI, export, and analysis code.
#[derive(Clone, Debug)]
pub struct DataFrame {
    row_count: usize,
    /// trial_id, in row-index order.
    trial_ids: Vec<u32>,
    /// 0-based trial.number within the study (in row-index order).
    trial_numbers: Vec<u32>,
    /// Numeric columns (name, values). Includes param / objective / user_attr / constraint / derived columns.
    numeric_cols: Vec<(String, Vec<f64>)>,
    /// String columns (name, values). Categorical param / user_attr string columns.
    string_cols: Vec<(String, Vec<String>)>,
    /// Parameter column names (in generation order).
    param_col_names: Vec<String>,
    objective_col_names: Vec<String>,
    user_attr_numeric_col_names: Vec<String>,
    user_attr_string_col_names: Vec<String>,
    constraint_col_names: Vec<String>,
    /// derived columns: is_feasible, constraint_sum 🟢
    derived_col_names: Vec<String>,
}

impl DataFrame {
    /// Returns an empty DataFrame with 0 rows and 0 columns.
    pub fn empty() -> Self {
        DataFrame {
            row_count: 0,
            trial_ids: vec![],
            trial_numbers: vec![],
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

    /// Builds a DataFrame from Trial row data.
    ///
    /// Columns are generated in the order param → objective → user_attr
    /// (numeric/string) → constraint → derived columns. A param becomes a
    /// string column if even one row has a category label, otherwise a
    /// numeric column. If constraints exist, the derived columns
    /// `is_feasible` / `constraint_sum` are added. Missing values are filled
    /// as: param: 0.0 / objective and user numeric: NaN / string: "" / constraint: 0.0.
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
        let trial_numbers: Vec<u32> = trial_rows.iter().map(|r| r.trial_number).collect();

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
            trial_numbers,
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

    /// Appends new trial rows to an existing DataFrame (for streaming loads / live updates).
    ///
    /// The resulting column contents match what you'd get by concatenating
    /// the existing rows' original data with `new_rows` and calling
    /// `from_trials` (only the internal column storage order may differ,
    /// which has no effect since lookups are by name). Rather than
    /// reconstructing everything by restoring rows to row-oriented form
    /// (an O(total rows) rebuild), columns are extended in place, so the
    /// cost is O(new_rows × column count).
    ///
    /// Pass the cumulative name lists (the full set including existing
    /// columns). A column that first appears partway through streaming is
    /// backfilled for existing rows with a default value (param numeric:
    /// 0.0 / objective and user numeric: NaN / string: "" / constraint: 0.0
    /// / is_feasible: 1.0). If a category label first appears on a numeric
    /// param column, the whole column is replaced with a string column, as
    /// in `from_trials` (existing rows become "").
    pub fn append_trials(
        &mut self,
        new_rows: &[TrialRow],
        param_names: &[String],
        objective_names: &[String],
        user_attr_numeric_names: &[String],
        user_attr_string_names: &[String],
        max_constraints: usize,
    ) {
        if new_rows.is_empty() {
            return;
        }
        let old_n = self.row_count;

        self.trial_ids.extend(new_rows.iter().map(|r| r.trial_id));
        self.trial_numbers
            .extend(new_rows.iter().map(|r| r.trial_number));

        // A queue, keyed by column name, of "not-yet-extended (length old_n)
        // same-named column indexes". Since the same name can occur across
        // multiple categories, entries are consumed from the front in
        // generation order (from_trials' param → objective → user →
        // constraint). This method processes in the same order, so the
        // correspondence is preserved. Uses a HashMap to avoid a linear scan
        // per name (the old implementation linearly scanned all columns on
        // every extension). Keys are owned copies of the column name
        // (because self.numeric_cols etc. are mutably borrowed in the loop).
        let mut numeric_pending: HashMap<String, VecDeque<usize>> = HashMap::new();
        for (i, (name, col)) in self.numeric_cols.iter().enumerate() {
            if col.len() == old_n {
                numeric_pending
                    .entry(name.clone())
                    .or_default()
                    .push_back(i);
            }
        }
        let mut string_pending: HashMap<String, VecDeque<usize>> = HashMap::new();
        for (i, (name, col)) in self.string_cols.iter().enumerate() {
            if col.len() == old_n {
                string_pending.entry(name.clone()).or_default().push_back(i);
            }
        }

        // Sets for membership checks against existing column names (replaces the old `iter().any()`).
        let mut param_name_set: std::collections::HashSet<String> =
            self.param_col_names.iter().cloned().collect();
        let mut objective_name_set: std::collections::HashSet<String> =
            self.objective_col_names.iter().cloned().collect();
        let mut uan_name_set: std::collections::HashSet<String> =
            self.user_attr_numeric_col_names.iter().cloned().collect();
        let mut uas_name_set: std::collections::HashSet<String> =
            self.user_attr_string_col_names.iter().cloned().collect();

        /// Extends the column at the pending queue's front index (no-op if absent).
        fn extend_numeric(
            cols: &mut [(String, Vec<f64>)],
            pending: &mut HashMap<String, VecDeque<usize>>,
            name: &str,
            values: impl Iterator<Item = f64>,
        ) {
            if let Some(idx) = pending.get_mut(name).and_then(VecDeque::pop_front) {
                cols[idx].1.extend(values);
            }
        }
        /// Extends the column at the pending queue's front index (no-op if absent).
        fn extend_string(
            cols: &mut [(String, Vec<String>)],
            pending: &mut HashMap<String, VecDeque<usize>>,
            name: &str,
            values: impl Iterator<Item = String>,
        ) {
            if let Some(idx) = pending.get_mut(name).and_then(VecDeque::pop_front) {
                cols[idx].1.extend(values);
            }
        }

        for name in param_names {
            let new_has_label = new_rows
                .iter()
                .any(|r| r.param_category_label.contains_key(name));
            let label_values = || {
                new_rows.iter().map(|r| {
                    r.param_category_label
                        .get(name)
                        .cloned()
                        .unwrap_or_default()
                })
            };
            if !param_name_set.contains(name) {
                // A param column that first appears partway through streaming. Existing rows are filled with the default.
                if new_has_label {
                    let mut vals = vec![String::new(); old_n];
                    vals.extend(label_values());
                    self.string_cols.push((name.clone(), vals));
                } else {
                    let mut vals = vec![0.0; old_n];
                    vals.extend(
                        new_rows
                            .iter()
                            .map(|r| *r.param_display.get(name).unwrap_or(&0.0)),
                    );
                    self.numeric_cols.push((name.clone(), vals));
                }
                self.param_col_names.push(name.clone());
                param_name_set.insert(name.clone());
            } else if string_pending
                .get(name.as_str())
                .is_some_and(|q| !q.is_empty())
            {
                extend_string(
                    &mut self.string_cols,
                    &mut string_pending,
                    name,
                    label_values(),
                );
            } else if new_has_label {
                // A category label first appears on a numeric column.
                // Since from_trials treats "the whole column as string if
                // even one row has a label", the column is replaced
                // (existing numeric rows become "").
                if let Some(idx) = numeric_pending
                    .get_mut(name.as_str())
                    .and_then(VecDeque::pop_front)
                {
                    self.numeric_cols.remove(idx);
                    // Removing shifts every column after idx one position
                    // forward, so correct the indexes stored in the pending queues.
                    for queue in numeric_pending.values_mut() {
                        for i in queue.iter_mut() {
                            if *i > idx {
                                *i -= 1;
                            }
                        }
                    }
                }
                let mut vals = vec![String::new(); old_n];
                vals.extend(label_values());
                self.string_cols.push((name.clone(), vals));
            } else {
                extend_numeric(
                    &mut self.numeric_cols,
                    &mut numeric_pending,
                    name,
                    new_rows
                        .iter()
                        .map(|r| *r.param_display.get(name).unwrap_or(&0.0)),
                );
            }
        }

        for (i, name) in objective_names.iter().enumerate() {
            let values = new_rows
                .iter()
                .map(move |r| r.objective_values.get(i).copied().unwrap_or(f64::NAN));
            if objective_name_set.contains(name) {
                extend_numeric(&mut self.numeric_cols, &mut numeric_pending, name, values);
            } else {
                let mut vals = vec![f64::NAN; old_n];
                vals.extend(values);
                self.numeric_cols.push((name.clone(), vals));
                self.objective_col_names.push(name.clone());
                objective_name_set.insert(name.clone());
            }
        }

        for name in user_attr_numeric_names {
            let values = new_rows
                .iter()
                .map(|r| *r.user_attrs_numeric.get(name).unwrap_or(&f64::NAN));
            if uan_name_set.contains(name) {
                extend_numeric(&mut self.numeric_cols, &mut numeric_pending, name, values);
            } else {
                let mut vals = vec![f64::NAN; old_n];
                vals.extend(values);
                self.numeric_cols.push((name.clone(), vals));
                self.user_attr_numeric_col_names.push(name.clone());
                uan_name_set.insert(name.clone());
            }
        }

        for name in user_attr_string_names {
            let values = new_rows
                .iter()
                .map(|r| r.user_attrs_string.get(name).cloned().unwrap_or_default());
            if uas_name_set.contains(name) {
                extend_string(&mut self.string_cols, &mut string_pending, name, values);
            } else {
                let mut vals = vec![String::new(); old_n];
                vals.extend(values);
                self.string_cols.push((name.clone(), vals));
                self.user_attr_string_col_names.push(name.clone());
                uas_name_set.insert(name.clone());
            }
        }

        // The constraint column count never shrinks (it may grow during streaming).
        let max_c = max_constraints.max(self.constraint_col_names.len());
        if max_c > 0 {
            for ci in 0..max_c {
                let col_name = format!("c{}", ci + 1);
                let values = new_rows
                    .iter()
                    .map(move |r| r.constraint_values.get(ci).copied().unwrap_or(0.0));
                if ci < self.constraint_col_names.len() {
                    extend_numeric(
                        &mut self.numeric_cols,
                        &mut numeric_pending,
                        &col_name,
                        values,
                    );
                } else {
                    let mut vals = vec![0.0; old_n];
                    vals.extend(values);
                    self.numeric_cols.push((col_name.clone(), vals));
                    self.constraint_col_names.push(col_name);
                }
            }

            // Derived columns. If constraints appear only partway through, existing rows are "no constraint" = feasible / sum 0.
            let feasible_values = new_rows.iter().map(|r| {
                if r.constraint_values.iter().all(|&c| c <= 0.0) {
                    1.0
                } else {
                    0.0
                }
            });
            if self.derived_col_names.iter().any(|n| n == "is_feasible") {
                extend_numeric(
                    &mut self.numeric_cols,
                    &mut numeric_pending,
                    "is_feasible",
                    feasible_values,
                );
            } else {
                let mut vals = vec![1.0; old_n];
                vals.extend(feasible_values);
                self.numeric_cols.push(("is_feasible".to_string(), vals));
                self.derived_col_names.push("is_feasible".to_string());
            }

            let sum_values = new_rows.iter().map(|r| r.constraint_values.iter().sum());
            if self.derived_col_names.iter().any(|n| n == "constraint_sum") {
                extend_numeric(
                    &mut self.numeric_cols,
                    &mut numeric_pending,
                    "constraint_sum",
                    sum_values,
                );
            } else {
                let mut vals = vec![0.0; old_n];
                vals.extend(sum_values);
                self.numeric_cols.push(("constraint_sum".to_string(), vals));
                self.derived_col_names.push("constraint_sum".to_string());
            }
        }

        self.row_count = old_n + new_rows.len();
        debug_assert!(
            self.numeric_cols
                .iter()
                .all(|(_, c)| c.len() == self.row_count)
                && self
                    .string_cols
                    .iter()
                    .all(|(_, c)| c.len() == self.row_count),
            "append_trials: column length mismatch after append"
        );
    }

    /// Returns the trial_id for the given row (`None` if out of range).
    pub fn get_trial_id(&self, row: usize) -> Option<u32> {
        self.trial_ids.get(row).copied()
    }

    /// Returns the 0-based trial.number within the study (Optuna's `trial.number`).
    /// Rows without a set value fall back to the row index.
    pub fn get_trial_number(&self, row: usize) -> Option<u32> {
        if row >= self.row_count {
            return None;
        }
        Some(self.trial_numbers.get(row).copied().unwrap_or(row as u32))
    }

    /// Parameter column names (in generation order).
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

    /// Returns the row count (number of trials).
    pub fn row_count(&self) -> usize {
        self.row_count
    }

    /// Returns all column names (numeric columns then string columns).
    pub fn column_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.numeric_cols.iter().map(|(n, _)| n.clone()).collect();
        names.extend(self.string_cols.iter().map(|(n, _)| n.clone()));
        names
    }

    /// Looks up a numeric column by name (the first one if duplicates exist; `None` if absent).
    pub fn get_numeric_column(&self, name: &str) -> Option<&[f64]> {
        self.numeric_cols
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.as_slice())
    }

    /// Looks up a string column by name (the first one if duplicates exist; `None` if absent).
    pub fn get_string_column(&self, name: &str) -> Option<&[String]> {
        self.string_cols
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.as_slice())
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

        let trial_numbers: Vec<u32> = self
            .trial_numbers
            .iter()
            .enumerate()
            .filter_map(|(i, &num)| {
                if mask.get(i).copied().unwrap_or(false) {
                    Some(num)
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
            trial_numbers,
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
