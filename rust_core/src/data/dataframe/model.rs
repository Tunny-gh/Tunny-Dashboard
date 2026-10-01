use std::collections::{BTreeMap, HashMap, VecDeque};

use serde_json::Value;

use super::feasibility::FeasibilityState;
use super::types::TrialRow;

fn append_json_attrs(
    columns: &mut BTreeMap<String, Vec<(usize, Value)>>,
    array_lengths: &mut BTreeMap<String, usize>,
    row_index: usize,
    row: &TrialRow,
) {
    let mut push = |name: &str, value: Value| {
        if let Value::Array(items) = &value {
            let max_len = array_lengths.entry(name.to_string()).or_default();
            *max_len = (*max_len).max(items.len());
        }
        columns
            .entry(name.to_string())
            .or_default()
            .push((row_index, value));
    };
    for (name, value) in &row.user_attrs_json {
        push(name, value.clone());
    }
    // Rows built outside JSON storage (including flat CSV) may only populate
    // the existing typed maps. Preserve their previous numeric/text behavior.
    for (name, &number) in &row.user_attrs_numeric {
        if !row.user_attrs_json.contains_key(name) {
            if let Some(number) = serde_json::Number::from_f64(number) {
                push(name, Value::Number(number));
            }
        }
    }
    for (name, text) in &row.user_attrs_string {
        if !row.user_attrs_json.contains_key(name)
            && !row
                .user_attrs_numeric
                .get(name)
                .is_some_and(|v| v.is_finite())
        {
            push(name, Value::String(text.clone()));
        }
    }
}

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
    /// Positions of user-attribute columns in the corresponding value vectors.
    /// Streaming can append new columns after other categories, so names alone
    /// cannot distinguish an attribute from a same-named parameter or objective.
    user_attr_numeric_col_indices: Vec<usize>,
    user_attr_string_col_indices: Vec<usize>,
    /// Present values only, in row order. Missing rows allocate no JSON value.
    user_attr_json_cols: BTreeMap<String, Vec<(usize, Value)>>,
    /// Maximum array length for each attribute key that has an array value.
    user_attr_array_lengths: BTreeMap<String, usize>,
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
            user_attr_numeric_col_indices: vec![],
            user_attr_string_col_indices: vec![],
            user_attr_json_cols: BTreeMap::new(),
            user_attr_array_lengths: BTreeMap::new(),
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
    /// as: numeric values: NaN / string: "".
    pub fn from_trials(
        trial_rows: &[TrialRow],
        param_names: &[String],
        objective_names: &[String],
        user_attr_numeric_names: &[String],
        user_attr_string_names: &[String],
        max_constraints: usize,
    ) -> Self {
        let n = trial_rows.len();
        let max_constraints = max_constraints.max(
            trial_rows
                .iter()
                .map(|r| r.constraint_values.len())
                .max()
                .unwrap_or(0),
        );
        if n == 0 && max_constraints == 0 {
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
        let mut user_attr_numeric_col_indices = Vec::new();
        let mut user_attr_string_col_indices = Vec::new();
        let mut user_attr_json_cols: BTreeMap<String, Vec<(usize, Value)>> = BTreeMap::new();
        let mut user_attr_array_lengths = BTreeMap::new();
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
                    .map(|r| *r.param_display.get(name).unwrap_or(&f64::NAN))
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
            user_attr_numeric_col_indices.push(numeric_cols.len());
            numeric_cols.push((name.clone(), vals));
            user_attr_numeric_col_names.push(name.clone());
        }

        for name in user_attr_string_names {
            let vals: Vec<String> = trial_rows
                .iter()
                .map(|r| r.user_attrs_string.get(name).cloned().unwrap_or_default())
                .collect();
            user_attr_string_col_indices.push(string_cols.len());
            string_cols.push((name.clone(), vals));
            user_attr_string_col_names.push(name.clone());
        }

        for (row_index, row) in trial_rows.iter().enumerate() {
            append_json_attrs(
                &mut user_attr_json_cols,
                &mut user_attr_array_lengths,
                row_index,
                row,
            );
        }

        if max_constraints > 0 {
            for ci in 0..max_constraints {
                let col_name = format!("c{}", ci + 1);
                let vals: Vec<f64> = trial_rows
                    .iter()
                    .map(|r| r.constraint_values.get(ci).copied().unwrap_or(f64::NAN))
                    .collect();
                numeric_cols.push((col_name.clone(), vals));
                constraint_col_names.push(col_name);
            }

            let is_feasible_vals: Vec<f64> = trial_rows
                .iter()
                .map(|r| {
                    FeasibilityState::classify(&r.constraint_values, max_constraints).numeric()
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
            user_attr_numeric_col_indices,
            user_attr_string_col_indices,
            user_attr_json_cols,
            user_attr_array_lengths,
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
    /// cost is O(new_rows × column count), except constraint schema growth
    /// reclassifies historical rows.
    ///
    /// Pass the cumulative name lists (the full set including existing
    /// columns). A column that first appears partway through streaming is
    /// backfilled for existing rows with a default value (numeric param,
    /// objective, user numeric and constraint: NaN / string: "").
    /// If a category label first appears on a numeric
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
        if new_rows.is_empty() && max_constraints <= self.constraint_col_names.len() {
            return;
        }
        let old_n = self.row_count;

        self.trial_ids.extend(new_rows.iter().map(|r| r.trial_id));
        self.trial_numbers
            .extend(new_rows.iter().map(|r| r.trial_number));
        for (offset, row) in new_rows.iter().enumerate() {
            append_json_attrs(
                &mut self.user_attr_json_cols,
                &mut self.user_attr_array_lengths,
                old_n + offset,
                row,
            );
        }

        // A queue, keyed by column name, of existing non-attribute columns
        // awaiting extension. Attribute columns use their recorded positions:
        // streaming can append a parameter after an existing same-named
        // attribute, so physical order does not establish category order.
        let mut numeric_pending: HashMap<String, VecDeque<usize>> = HashMap::new();
        let numeric_attr_positions: std::collections::HashSet<usize> =
            self.user_attr_numeric_col_indices.iter().copied().collect();
        for (i, (name, col)) in self.numeric_cols.iter().enumerate() {
            if col.len() == old_n && !numeric_attr_positions.contains(&i) {
                numeric_pending
                    .entry(name.clone())
                    .or_default()
                    .push_back(i);
            }
        }
        let mut string_pending: HashMap<String, VecDeque<usize>> = HashMap::new();
        let string_attr_positions: std::collections::HashSet<usize> =
            self.user_attr_string_col_indices.iter().copied().collect();
        for (i, (name, col)) in self.string_cols.iter().enumerate() {
            if col.len() == old_n && !string_attr_positions.contains(&i) {
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
                    let mut vals = vec![f64::NAN; old_n];
                    vals.extend(
                        new_rows
                            .iter()
                            .map(|r| *r.param_display.get(name).unwrap_or(&f64::NAN)),
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
                    for attr_idx in &mut self.user_attr_numeric_col_indices {
                        if *attr_idx > idx {
                            *attr_idx -= 1;
                        }
                    }
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
                        .map(|r| *r.param_display.get(name).unwrap_or(&f64::NAN)),
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
                if let Some(position) = self
                    .user_attr_numeric_col_names
                    .iter()
                    .position(|existing| existing == name)
                {
                    let idx = self.user_attr_numeric_col_indices[position];
                    self.numeric_cols[idx].1.extend(values);
                }
            } else {
                let mut vals = vec![f64::NAN; old_n];
                vals.extend(values);
                self.user_attr_numeric_col_indices
                    .push(self.numeric_cols.len());
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
                if let Some(position) = self
                    .user_attr_string_col_names
                    .iter()
                    .position(|existing| existing == name)
                {
                    let idx = self.user_attr_string_col_indices[position];
                    self.string_cols[idx].1.extend(values);
                }
            } else {
                let mut vals = vec![String::new(); old_n];
                vals.extend(values);
                self.user_attr_string_col_indices
                    .push(self.string_cols.len());
                self.string_cols.push((name.clone(), vals));
                self.user_attr_string_col_names.push(name.clone());
                uas_name_set.insert(name.clone());
            }
        }

        // The constraint column count never shrinks (it may grow during streaming).
        let old_c = self.constraint_col_names.len();
        let was_constrained = self.feasibility().has_constraints();
        let max_c = max_constraints.max(old_c).max(
            new_rows
                .iter()
                .map(|r| r.constraint_values.len())
                .max()
                .unwrap_or(0),
        );
        if max_c > 0 || was_constrained {
            for ci in 0..max_c {
                let col_name = format!("c{}", ci + 1);
                let values = new_rows
                    .iter()
                    .map(move |r| r.constraint_values.get(ci).copied().unwrap_or(f64::NAN));
                if ci < self.constraint_col_names.len() {
                    extend_numeric(
                        &mut self.numeric_cols,
                        &mut numeric_pending,
                        &col_name,
                        values,
                    );
                } else {
                    let mut vals = vec![f64::NAN; old_n];
                    vals.extend(values);
                    self.numeric_cols.push((col_name.clone(), vals));
                    self.constraint_col_names.push(col_name);
                }
            }

            // Newly constrained historical rows have no verified evaluation.
            let feasible_values = new_rows
                .iter()
                .map(|r| FeasibilityState::classify(&r.constraint_values, max_c).numeric());
            if self.derived_col_names.iter().any(|n| n == "is_feasible") {
                extend_numeric(
                    &mut self.numeric_cols,
                    &mut numeric_pending,
                    "is_feasible",
                    feasible_values,
                );
            } else {
                let mut vals = vec![f64::NAN; old_n];
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
        if max_c != old_c {
            self.mark_constrained();
        }
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

    /// Records attribute-only constrained studies and reclassifies against the
    /// current schema, including historical rows after schema growth.
    pub fn mark_constrained(&mut self) {
        let expected = self.constraint_col_names.len();
        let states: Vec<f64> = (0..self.row_count)
            .map(|row| {
                let values: Vec<f64> = self
                    .constraint_col_names
                    .iter()
                    .map(|name| self.get_numeric_column(name).unwrap()[row])
                    .collect();
                FeasibilityState::classify(&values, expected).numeric()
            })
            .collect();
        if let Some((_, col)) = self
            .numeric_cols
            .iter_mut()
            .find(|(name, _)| name == "is_feasible")
        {
            *col = states;
        } else {
            self.numeric_cols.push(("is_feasible".into(), states));
            self.derived_col_names.push("is_feasible".into());
        }
        if self.get_numeric_column("constraint_sum").is_none() {
            self.numeric_cols
                .push(("constraint_sum".into(), vec![0.0; self.row_count]));
            self.derived_col_names.push("constraint_sum".into());
        }
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

    /// User attribute columns in category order, safe when another category
    /// contains a column with the same name.
    pub fn user_attr_numeric_columns(&self) -> impl Iterator<Item = (&str, &[f64])> {
        self.user_attr_numeric_col_names
            .iter()
            .zip(&self.user_attr_numeric_col_indices)
            .map(|(name, &idx)| (name.as_str(), self.numeric_cols[idx].1.as_slice()))
    }

    pub fn user_attr_string_columns(&self) -> impl Iterator<Item = (&str, &[String])> {
        self.user_attr_string_col_names
            .iter()
            .zip(&self.user_attr_string_col_indices)
            .map(|(name, &idx)| (name.as_str(), self.string_cols[idx].1.as_slice()))
    }

    /// Keys with at least one present JSON value, in deterministic order.
    pub fn user_attr_names(&self) -> impl Iterator<Item = &str> {
        self.user_attr_json_cols.keys().map(String::as_str)
    }

    /// The original typed value for one key and row. `None` means absent;
    /// `Some(Value::Null)` means explicitly present as JSON null.
    pub fn user_attr_value(&self, name: &str, row: usize) -> Option<&Value> {
        let values = self.user_attr_json_cols.get(name)?;
        let index = values
            .binary_search_by_key(&row, |(index, _)| *index)
            .ok()?;
        Some(&values[index].1)
    }

    /// Maximum length of an array stored under this key; `None` means no array value.
    pub fn user_attr_array_len(&self, name: &str) -> Option<usize> {
        self.user_attr_array_lengths.get(name).copied()
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

        let mut next_row = 0;
        let row_map: Vec<Option<usize>> = (0..self.row_count)
            .map(|index| {
                mask.get(index).copied().unwrap_or(false).then(|| {
                    let current = next_row;
                    next_row += 1;
                    current
                })
            })
            .collect();
        let user_attr_json_cols: BTreeMap<String, Vec<(usize, Value)>> = self
            .user_attr_json_cols
            .iter()
            .map(|(name, values)| {
                let filtered = values
                    .iter()
                    .filter_map(|(index, value)| {
                        row_map[*index].map(|new_index| (new_index, value.clone()))
                    })
                    .collect();
                (name.clone(), filtered)
            })
            .collect();
        let user_attr_array_lengths = self
            .user_attr_array_lengths
            .keys()
            .filter_map(|name| {
                let max_len = user_attr_json_cols
                    .get(name)?
                    .iter()
                    .filter_map(|(_, value)| value.as_array().map(Vec::len))
                    .max()?;
                Some((name.clone(), max_len))
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
            user_attr_numeric_col_indices: self.user_attr_numeric_col_indices.clone(),
            user_attr_string_col_indices: self.user_attr_string_col_indices.clone(),
            user_attr_json_cols,
            user_attr_array_lengths,
            constraint_col_names: self.constraint_col_names.clone(),
            derived_col_names: self.derived_col_names.clone(),
        }
    }
}
