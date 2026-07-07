use std::collections::{HashMap, VecDeque};

use super::types::TrialRow;

/// 列指向の Trial テーブル（数値列・文字列列を名前で引く軽量 DataFrame）。
/// journal / RDB パーサが構築し、UI・エクスポート・分析コードが共通で参照する。
#[derive(Clone, Debug)]
pub struct DataFrame {
    row_count: usize,
    /// 行 index 順の trial_id。
    trial_ids: Vec<u32>,
    /// Study 内 0 始まりの trial.number（行 index 順）。
    trial_numbers: Vec<u32>,
    /// 数値列（名前, 値）。param / objective / user_attr / constraint / 派生列を含む。
    numeric_cols: Vec<(String, Vec<f64>)>,
    /// 文字列列（名前, 値）。カテゴリカル param / user_attr 文字列列。
    string_cols: Vec<(String, Vec<String>)>,
    /// パラメータ列名（生成順）。
    param_col_names: Vec<String>,
    objective_col_names: Vec<String>,
    user_attr_numeric_col_names: Vec<String>,
    user_attr_string_col_names: Vec<String>,
    constraint_col_names: Vec<String>,
    /// derived columns: is_feasible, constraint_sum 🟢
    derived_col_names: Vec<String>,
}

impl DataFrame {
    /// 行 0・列 0 の空 DataFrame を返す。
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

    /// Trial 行データから DataFrame を構築する。
    ///
    /// 列の生成順は param → objective → user_attr（数値/文字列）→ constraint → 派生列。
    /// param はカテゴリラベルが 1 行でもあれば文字列列、なければ数値列になる。
    /// 制約がある場合は派生列 `is_feasible` / `constraint_sum` を追加する。
    /// 欠損値は param: 0.0 / objective・user 数値: NaN / 文字列: "" / 制約: 0.0 で埋める。
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

    /// 既存 DataFrame へ新規 trial 行を追記する（ストリーミングロード・ライブ更新用）。
    ///
    /// 列内容は「既存行の元データと `new_rows` を連結して `from_trials` を呼んだ場合」と
    /// 一致する（内部の列格納順のみ異なりうるが、参照は名前引きのため影響しない）。
    /// 行を行指向に復元して全体を作り直す O(全行数) の再構築を避け、列を in-place で
    /// 伸長するため、コストは O(new_rows × 列数) で済む。
    ///
    /// 名前リストは累積（既存列を含む全体）を渡すこと。ストリーミング途中で初出現した
    /// 列は既存行ぶんをデフォルト値でバックフィルする（param 数値: 0.0 /
    /// objective・user 数値: NaN / 文字列: "" / 制約: 0.0 / is_feasible: 1.0）。
    /// 数値 param 列にカテゴリラベルが初出現した場合は `from_trials` と同様に
    /// 列全体を文字列列へ置き換える（既存行は ""）。
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

        // 列名 → 「まだ伸長していない（長さ old_n の）同名列 index」のキュー。
        // 同名列が複数カテゴリに存在しうるため、生成順（from_trials の
        // param → objective → user → constraint）で先頭から消費する。本メソッドの
        // 処理順が同じなので対応関係が保たれる。名前ごとの線形探索を避けるための
        // HashMap 化（旧実装は伸長のたびに全列を線形走査していた）。
        // キーは列名のコピーで持つ（ループ中に self.numeric_cols 等を可変借用するため）。
        let mut numeric_pending: HashMap<String, VecDeque<usize>> = HashMap::new();
        for (i, (name, col)) in self.numeric_cols.iter().enumerate() {
            if col.len() == old_n {
                numeric_pending.entry(name.clone()).or_default().push_back(i);
            }
        }
        let mut string_pending: HashMap<String, VecDeque<usize>> = HashMap::new();
        for (i, (name, col)) in self.string_cols.iter().enumerate() {
            if col.len() == old_n {
                string_pending.entry(name.clone()).or_default().push_back(i);
            }
        }

        // 既存列名の membership 判定用セット（旧実装の `iter().any()` の置き換え）。
        let mut param_name_set: std::collections::HashSet<String> =
            self.param_col_names.iter().cloned().collect();
        let mut objective_name_set: std::collections::HashSet<String> =
            self.objective_col_names.iter().cloned().collect();
        let mut uan_name_set: std::collections::HashSet<String> =
            self.user_attr_numeric_col_names.iter().cloned().collect();
        let mut uas_name_set: std::collections::HashSet<String> =
            self.user_attr_string_col_names.iter().cloned().collect();

        /// pending キューの先頭 index の列を伸長する（無ければ no-op）。
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
        /// pending キューの先頭 index の列を伸長する（無ければ no-op）。
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
                // ストリーミング途中で初出現した param 列。既存行はデフォルトで埋める。
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
                // 数値列にカテゴリラベルが初出現。from_trials は「1 行でもラベルがあれば
                // 列全体を文字列」とするため、列を置き換える（既存の数値行は ""）。
                if let Some(idx) = numeric_pending
                    .get_mut(name.as_str())
                    .and_then(VecDeque::pop_front)
                {
                    self.numeric_cols.remove(idx);
                    // remove で idx より後ろの列が 1 つずつ前へ詰まるため、
                    // pending キュー内の index を補正する。
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

        // 制約列数は縮まない（ストリーミング中に増えることはある）。
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

            // 派生列。制約が途中から現れた場合、既存行は「制約なし」= feasible / 合計 0。
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

    /// 指定行の trial_id を返す（範囲外は `None`）。
    pub fn get_trial_id(&self, row: usize) -> Option<u32> {
        self.trial_ids.get(row).copied()
    }

    /// Study 内 0 始まりの trial.number（Optuna の `trial.number`）を返す。
    /// 値が未設定の行は行 index にフォールバックする。
    pub fn get_trial_number(&self, row: usize) -> Option<u32> {
        if row >= self.row_count {
            return None;
        }
        Some(self.trial_numbers.get(row).copied().unwrap_or(row as u32))
    }

    /// パラメータ列名（生成順）。
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

    /// 行数（trial 数）を返す。
    pub fn row_count(&self) -> usize {
        self.row_count
    }

    /// 全列名（数値列 → 文字列列の順）を返す。
    pub fn column_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.numeric_cols.iter().map(|(n, _)| n.clone()).collect();
        names.extend(self.string_cols.iter().map(|(n, _)| n.clone()));
        names
    }

    /// 数値列を名前で引く（同名列が複数ある場合は先頭、無ければ `None`）。
    pub fn get_numeric_column(&self, name: &str) -> Option<&[f64]> {
        self.numeric_cols
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.as_slice())
    }

    /// 文字列列を名前で引く（同名列が複数ある場合は先頭、無ければ `None`）。
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
