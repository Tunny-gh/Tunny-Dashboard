use std::collections::{HashMap, HashSet};

use super::types::OptimizationDirection;

/// journal のイベント列から `StudyMeta` を組み立てる途中状態。
pub(super) struct StudyBuilder {
    pub(super) study_id: u32,
    pub(super) name: String,
    pub(super) directions: Vec<OptimizationDirection>,
    pub(super) total_trials: u32,
    pub(super) completed_trials: u32,
    pub(super) param_names: HashSet<String>,
    pub(super) objective_names: Vec<String>,
    pub(super) user_attr_names: HashSet<String>,
    pub(super) has_constraints: bool,
    /// パラメータごとの宣言レンジ (low, high)（表示単位、初出の分布から記録）。
    /// 数値パラメータ（Float / Int）のみ。サロゲート最適化の探索範囲に使う。
    pub(super) param_bounds: HashMap<String, (f64, f64)>,
}

/// journal のイベント列から trial 1 件分のデータを組み立てる途中状態。
pub(super) struct TrialBuilder {
    pub(super) study_id: u32,
    /// Study 内 0 始まりの trial.number（作成順 = op_code=4 の study 内出現順）。
    pub(super) trial_number: u32,
    pub(super) state: u8,
    pub(super) values: Option<Vec<f64>>,
    pub(super) param_display: HashMap<String, f64>,
    pub(super) param_category_label: HashMap<String, String>,
    pub(super) user_attrs_numeric: HashMap<String, f64>,
    pub(super) user_attrs_string: HashMap<String, String>,
    pub(super) constraint_values: Vec<f64>,
    pub(super) has_constraints: bool,
    /// 開始日時。unix 秒（naive）。op_code=4 の `datetime_start` 由来。
    pub(super) datetime_start: Option<f64>,
    /// 完了日時。unix 秒（naive）。op_code=6 の `datetime_complete` 由来。
    pub(super) datetime_complete: Option<f64>,
    /// 中間値 `(step, value)`。op_code=7 由来。挿入順（後で step 昇順にソートする）。
    pub(super) intermediate_values: Vec<(u64, f64)>,
}
