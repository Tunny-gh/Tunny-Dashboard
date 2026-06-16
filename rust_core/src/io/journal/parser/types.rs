/// Documentation.
#[derive(Debug, Clone, PartialEq)]
pub enum OptimizationDirection {
    Minimize,
    Maximize,
}

/// Documentation.
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

/// Documentation.
#[derive(Debug)]
pub struct ParseResult {
    pub studies: Vec<StudyMeta>,
    pub duration_ms: f64,
}

pub struct JournalParser;
