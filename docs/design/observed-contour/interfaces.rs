// Observed Contour インターフェース定義（設計用スケッチ。実コードは各クレートに実装）
// 関連: architecture.md / dataflow.md

// ============================================================
// rust_core: 新モジュール contour
//   rust_core/src/contour/mod.rs（lib.rs に `pub mod contour;`）
// ============================================================

/// 観測点だけから補間した格子。データの無いセルは None（マスク＝外挿しない）。
pub struct ObservedSurface {
    /// X 軸格子（観測 X 範囲の linspace）。
    pub x_values: Vec<f64>,
    /// Y 軸格子（観測 Y 範囲の linspace）。
    pub y_values: Vec<f64>,
    /// 補間値の格子。`z[i][j]` は (x_values[i], y_values[j]) の値。
    /// None = 凸包外 / 疎ガードで落ちた三角形領域（データなし）。
    pub z: Vec<Vec<Option<f64>>>,
}

/// 観測点 (x, y, value) から、凸包内のみを Delaunay 線形補間した格子を返す。
///
/// - `n_grid`: 一辺の格子点数（例 60）。
/// - `max_edge_ratio`: 疎ガード。三角形の最長辺が `max_edge_ratio * bbox対角` を超えると
///   その三角形を捨てる（離れたクラスタを偽の面で繋がない）。0.0 で無効、典型 0.1〜0.3。
///
/// 点が 3 未満 / 共線 / 退化のときは全セル None の格子を返す（panic しない）。
pub fn observed_surface(pts: &[[f64; 3]], n_grid: usize, max_edge_ratio: f64) -> ObservedSurface;

// ============================================================
// egui-app: messages.rs
// ============================================================

pub struct ObservedContourResult {
    pub x_name: String,
    pub y_name: String,
    pub value_name: String,
    pub surface: tunny_core::contour::ObservedSurface,
    /// 重畳表示用の観測点（feasible フィルタ適用済み）。[x, y, value]。
    pub points: Vec<[f64; 3]>,
}

pub enum AppMessage {
    // ... 既存 ...
    ObservedContourDone(ObservedContourResult),
    ObservedContourFailed(String),
}

// ============================================================
// egui-app: widget_states.rs
// ============================================================

pub struct ObservedContourComputeRequest {
    pub x: String,
    pub y: String,
    pub value: String,
    pub n_grid: usize,
    pub max_edge_ratio: f64,
    pub feasible_only: bool,
}

pub struct ObservedContourState {
    pub selected_x: String,
    pub selected_y: String,
    /// 値（色）に使う列名。params∪objectives から選択。
    pub selected_value: String,
    /// 疎ガード閾値（Coverage スライダー）。
    pub max_edge_ratio: f64,
    pub show_points: bool,
    pub feasible_only: bool,
    pub log_scale: bool,        // Phase 2（色の対数スケール）
    pub show_contour_lines: bool, // Phase 2
    pub computing: bool,
    pub result: Option<crate::state::messages::ObservedContourResult>,
    pub error_message: Option<String>,
    pub pending_compute: Option<ObservedContourComputeRequest>,
    pub detail_modal: crate::ui::widgets::trial_detail_modal::TrialDetailModal, // Phase 2
}

impl ObservedContourState {
    /// グローバル widget の計算実行状態・結果・エラーを取り込む（キャンバス各アイテム伝播用）。
    pub fn adopt_compute_state(&mut self, src: &Self);
}

// ============================================================
// egui-app: common/heatmap.rs（マスク対応の追加）
// ============================================================

/// `None` セルを塗らない（パネル背景のまま）ヒートマップ描画。
pub fn draw_heatmap_masked(
    painter: &egui::Painter,
    rect: egui::Rect,
    values: &[Vec<Option<f64>>],
    cmap: crate::theme::colormap::ColorMap,
);

// ============================================================
// egui-app: ウィジェット本体 observed_contour::show
//   egui-app/src/ui/widgets/pdp/observed_contour.rs（または新 group）
// ============================================================

#[allow(clippy::too_many_arguments)]
pub fn show(
    ui: &mut egui::Ui,
    state: &mut ObservedContourState,
    column_names: &[String], // params ∪ objectives（数値列のみ）
    cmap: crate::theme::colormap::ColorMap,
    view: &crate::state::types::StudyView, // 点重畳・クリック用
    has_constraints: bool,
);

// ============================================================
// egui-app: layout_state.rs
// ============================================================
// ChartId に `ObservedContour` を追加（label 例 "Observed Contour"）。
// right_panel.rs の "Variable Analysis" 群へ PanelItem を追加。
