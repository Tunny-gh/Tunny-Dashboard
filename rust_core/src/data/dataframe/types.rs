use std::collections::HashMap;

/// 1 trial 分のデータを保持する行。DataFrame 構築の入力となる中間表現。
#[derive(Clone)]
pub struct TrialRow {
    /// ストレージ横断のグローバル trial_id（op_code=4 出現順）。
    pub trial_id: u32,
    /// Study 内 0 始まりの trial.number（Optuna の `trial.number`、作成順）。
    pub trial_number: u32,
    /// パラメータ名から表示値（数値）へのマップ。
    pub param_display: HashMap<String, f64>,
    /// カテゴリカルパラメータ名から選択肢ラベル文字列へのマップ。
    pub param_category_label: HashMap<String, String>,
    /// objectivevalue list（obj0, obj1, ...）
    pub objective_values: Vec<f64>,
    /// user_attr numeric type（REQ-012）
    pub user_attrs_numeric: HashMap<String, f64>,
    /// user_attr string type（REQ-012）
    pub user_attrs_string: HashMap<String, String>,
    /// constraints value list（REQ-013）
    pub constraint_values: Vec<f64>,
}
