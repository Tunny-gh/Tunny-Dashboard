/// 最適化の方向（最小化 / 最大化）。
#[derive(Debug, Clone, PartialEq)]
pub enum OptimizationDirection {
    Minimize,
    Maximize,
}

/// Study のメタ情報（パラメータ・目的・attribute の名前一覧や試行数など）。
#[derive(Debug, Clone)]
pub struct StudyMeta {
    pub study_id: u32,
    pub name: String,
    pub directions: Vec<OptimizationDirection>,
    pub completed_trials: u32,
    pub total_trials: u32,
    pub param_names: Vec<String>,
    pub objective_names: Vec<String>,
    pub user_attr_names: Vec<String>,
    pub has_constraints: bool,
    /// パラメータごとの宣言レンジ (low, high)（表示単位、数値パラメータのみ）。
    /// log に記載された探索空間の範囲。サロゲート最適化の探索箱に使う。
    pub param_bounds: std::collections::HashMap<String, (f64, f64)>,
}

/// journal ファイルのパース結果（全 Study のメタ情報と所要時間）。
#[derive(Debug)]
pub struct ParseResult {
    pub studies: Vec<StudyMeta>,
    pub duration_ms: f64,
}
