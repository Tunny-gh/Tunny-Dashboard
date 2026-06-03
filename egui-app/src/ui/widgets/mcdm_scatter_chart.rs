//! MCDM Scatter Chart Widget

use crate::state::results::{McdmMethod, McdmResult};
use crate::state::types::StudyView;
use crate::theme::chart_colors::{
    COLOR_EMPTY_STATE, COLOR_MCDM_HIGH, COLOR_MCDM_LOW, COLOR_MCDM_MID, COLOR_MCDM_NONE,
};
use crate::theme::ERROR_COLOR;
use egui::Color32;

/// 軸識別子定数（get_axis_options と extract_axis_values で共有）
const AXIS_VIKOR_Q: &str = "VIKOR_Q";
const AXIS_VIKOR_S: &str = "VIKOR_S";
const AXIS_VIKOR_R: &str = "VIKOR_R";
const AXIS_TOPSIS_SCORE: &str = "TOPSIS_Score";
const AXIS_PHI_PLUS: &str = "Phi+";
const AXIS_PHI_MINUS: &str = "Phi-";
const AXIS_PHI_NET: &str = "Phi_Net";

/// 上位N件の色分け閾値
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ScatterTopN {
    Top5,
    Top10,
    Top20,
}

impl ScatterTopN {
    pub fn label(self) -> &'static str {
        match self {
            ScatterTopN::Top5 => "Top 5",
            ScatterTopN::Top10 => "Top 10",
            ScatterTopN::Top20 => "Top 20",
        }
    }

    pub fn all() -> &'static [ScatterTopN] {
        &[ScatterTopN::Top5, ScatterTopN::Top10, ScatterTopN::Top20]
    }
}

/// 軸選択オプション
#[derive(Clone, Debug)]
pub(crate) struct AxisOption {
    pub id: String,
    pub label: String,
}

/// 散布図計算メタデータ
#[derive(Clone, Debug)]
pub(crate) struct ScatterMetadata {
    pub total_trials: usize,
    pub compute_time_ms: f64,
}

/// キャッシュキー
#[derive(Clone, PartialEq, Eq)]
struct CacheKey {
    trial_count: usize,
    x_axis: String,
    y_axis: String,
    color_threshold: ScatterTopN,
    /// MCDM手法（手法切替を検知）
    result_method: McdmMethod,
    /// 先頭スコアのビット表現（重み変更を検知）
    result_score0_bits: u64,
    /// スコア件数
    result_score_count: usize,
}

/// MCDM 散布図ウィジェット
pub struct McdmScatterChart {
    /// X軸の軸識別子
    pub x_axis: String,
    /// Y軸の軸識別子
    pub y_axis: String,
    /// 色分け閾値
    pub color_threshold: ScatterTopN,
    /// 実行不可能解を表示するか（制約あり Study でのみ有効）
    pub show_infeasible: bool,
    // --- 内部キャッシュ状態 ---
    display_rows_cache: Option<Vec<(f64, f64, Color32)>>,
    infeasible_cache: Option<Vec<(f64, f64)>>,
    metadata: Option<ScatterMetadata>,
    error_message: Option<String>,
    cache_key: Option<CacheKey>,
}

impl Default for McdmScatterChart {
    fn default() -> Self {
        Self {
            x_axis: "Objective0".to_string(),
            y_axis: "Objective1".to_string(),
            color_threshold: ScatterTopN::Top10,
            show_infeasible: true,
            display_rows_cache: None,
            infeasible_cache: None,
            metadata: None,
            error_message: None,
            cache_key: None,
        }
    }
}

impl McdmScatterChart {
    /// 新規インスタンスを生成する
    pub fn new() -> Self {
        Self::default()
    }

    /// キャッシュを無効化する
    pub fn invalidate_cache(&mut self) {
        self.display_rows_cache = None;
        self.infeasible_cache = None;
        self.cache_key = None;
        self.error_message = None;
    }

    /// 現在の設定からキャッシュキーを生成する
    fn make_cache_key(&self, trial_count: usize, result: &McdmResult) -> CacheKey {
        let scores = result.primary_scores();
        CacheKey {
            trial_count,
            x_axis: self.x_axis.clone(),
            y_axis: self.y_axis.clone(),
            color_threshold: self.color_threshold,
            result_method: result.method(),
            result_score0_bits: scores.first().copied().unwrap_or(0.0).to_bits(),
            result_score_count: scores.len(),
        }
    }

    /// キャッシュが陳腐化しているか確認する
    fn is_cache_stale(&self, trial_count: usize, result: &McdmResult) -> bool {
        let scores = result.primary_scores();
        let score0_bits = scores.first().copied().unwrap_or(0.0).to_bits();
        match &self.cache_key {
            None => true,
            Some(key) => {
                key.trial_count != trial_count
                    || key.x_axis != self.x_axis
                    || key.y_axis != self.y_axis
                    || key.color_threshold != self.color_threshold
                    || key.result_method != result.method()
                    || key.result_score0_bits != score0_bits
                    || key.result_score_count != scores.len()
            }
        }
    }

    /// ウィジェットを描画する
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        mcdm_result: &Option<McdmResult>,
        view: &StudyView,
        obj_names: &[String],
    ) {
        let Some(result) = mcdm_result else {
            ui.centered_and_justified(|ui| {
                ui.colored_label(
                    COLOR_EMPTY_STATE,
                    "Run MCDM analysis first (Ranking tab → Run button)",
                );
            });
            return;
        };

        let options = get_axis_options(result, obj_names);

        // デフォルト軸が無効な場合に更新
        if !options.iter().any(|o| o.id == self.x_axis) {
            if let Some(first) = options.first() {
                self.x_axis = first.id.clone();
            }
        }
        if !options.iter().any(|o| o.id == self.y_axis) {
            if options.len() > 1 {
                self.y_axis = options[1].id.clone();
            } else if let Some(first) = options.first() {
                self.y_axis = first.id.clone();
            }
        }

        ui.horizontal(|ui| {
            ui.label("X:");
            egui::ComboBox::from_id_salt("mcdm_scatter_x_axis")
                .selected_text(&self.x_axis)
                .show_ui(ui, |ui| {
                    for opt in &options {
                        ui.selectable_value(&mut self.x_axis, opt.id.clone(), &opt.label);
                    }
                });

            ui.label("Y:");
            egui::ComboBox::from_id_salt("mcdm_scatter_y_axis")
                .selected_text(&self.y_axis)
                .show_ui(ui, |ui| {
                    for opt in &options {
                        ui.selectable_value(&mut self.y_axis, opt.id.clone(), &opt.label);
                    }
                });

            ui.label("Highlight:");
            egui::ComboBox::from_id_salt("mcdm_scatter_threshold")
                .selected_text(self.color_threshold.label())
                .show_ui(ui, |ui| {
                    for t in ScatterTopN::all() {
                        ui.selectable_value(&mut self.color_threshold, *t, t.label());
                    }
                });
        });

        let n_trials = view.row_count();
        let has_constraints = view.numeric_column("is_feasible").is_some();

        // キャッシュが陳腐化している場合に再計算
        if self.is_cache_stale(n_trials, result) {
            let new_key = self.make_cache_key(n_trials, result);
            let start = std::time::Instant::now();

            match compute_scatter_points(
                result,
                view,
                obj_names,
                &self.x_axis,
                &self.y_axis,
                self.color_threshold,
            ) {
                Ok((points, infeasible, mut meta)) => {
                    meta.compute_time_ms = start.elapsed().as_secs_f64() * 1000.0;
                    self.display_rows_cache = Some(points);
                    self.infeasible_cache = Some(infeasible);
                    self.cache_key = Some(new_key);
                    self.metadata = Some(meta);
                    self.error_message = None;
                }
                Err(e) => {
                    self.error_message = Some(e);
                    self.display_rows_cache = None;
                    self.infeasible_cache = None;
                    self.cache_key = None;
                }
            }
        }

        if let Some(ref error) = self.error_message {
            ui.colored_label(ERROR_COLOR, error);
            return;
        }

        if has_constraints {
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.show_infeasible, "Show Infeasible");
            });
        }

        let empty = vec![];
        let infeasible = self.infeasible_cache.as_deref().unwrap_or(&empty);
        if let Some(ref points) = self.display_rows_cache {
            render_scatter_plot(
                ui,
                points,
                infeasible,
                self.show_infeasible,
                &self.x_axis,
                &self.y_axis,
            );
        }

        ui.separator();
        if let Some(ref meta) = self.metadata {
            ui.label(
                egui::RichText::new(format!(
                    "Rendering {} points ({:.1}ms)",
                    meta.total_trials, meta.compute_time_ms
                ))
                .small(),
            );
        }
    }
}

// ──────────────────────────────────────────────────────────────
// 散布図レンダリング
// ──────────────────────────────────────────────────────────────

fn render_scatter_plot(
    ui: &mut egui::Ui,
    points: &[(f64, f64, Color32)],
    infeasible: &[(f64, f64)],
    show_infeasible: bool,
    x_label: &str,
    y_label: &str,
) {
    // 色別にグループ分け（レンダリング順: gray→yellow→orange→red）
    let top5 = 5_usize.min(points.len());
    let top10 = 10_usize.min(points.len());
    let top20 = 20_usize.min(points.len());
    let mut gray_pts: Vec<[f64; 2]> = Vec::with_capacity(points.len());
    let mut yellow_pts: Vec<[f64; 2]> = Vec::with_capacity(top20 - top10);
    let mut orange_pts: Vec<[f64; 2]> = Vec::with_capacity(top10 - top5);
    let mut red_pts: Vec<[f64; 2]> = Vec::with_capacity(top5);

    for &(x, y, color) in points {
        let pt = [x, y];
        if color == COLOR_MCDM_HIGH {
            red_pts.push(pt);
        } else if color == COLOR_MCDM_MID {
            orange_pts.push(pt);
        } else if color == COLOR_MCDM_LOW {
            yellow_pts.push(pt);
        } else {
            gray_pts.push(pt);
        }
    }

    egui_plot::Plot::new("mcdm_scatter_plot")
        .x_axis_label(x_label)
        .y_axis_label(y_label)
        .legend(egui_plot::Legend::default())
        .show(ui, |plot_ui| {
            // 実行不可能解を最背面に描画
            if show_infeasible && !infeasible.is_empty() {
                let pts: Vec<[f64; 2]> = infeasible.iter().map(|&(x, y)| [x, y]).collect();
                plot_ui.points(
                    egui_plot::Points::new(pts)
                        .name("Infeasible")
                        .color(crate::theme::chart_colors::COLOR_INFEASIBLE)
                        .radius(3.0),
                );
            }
            if !gray_pts.is_empty() {
                plot_ui.points(
                    egui_plot::Points::new(gray_pts)
                        .name("Others")
                        .color(COLOR_MCDM_NONE)
                        .radius(3.0),
                );
            }
            if !yellow_pts.is_empty() {
                plot_ui.points(
                    egui_plot::Points::new(yellow_pts)
                        .name("Top 20")
                        .color(COLOR_MCDM_LOW)
                        .radius(4.0),
                );
            }
            if !orange_pts.is_empty() {
                plot_ui.points(
                    egui_plot::Points::new(orange_pts)
                        .name("Top 10")
                        .color(COLOR_MCDM_MID)
                        .radius(4.5),
                );
            }
            if !red_pts.is_empty() {
                plot_ui.points(
                    egui_plot::Points::new(red_pts)
                        .name("Top 5")
                        .color(COLOR_MCDM_HIGH)
                        .radius(5.0),
                );
            }
        });
}

// ──────────────────────────────────────────────────────────────
// 軸オプション生成
// ──────────────────────────────────────────────────────────────

/// MCDM結果から利用可能な軸オプションを生成する
pub(crate) fn get_axis_options(mcdm_result: &McdmResult, obj_names: &[String]) -> Vec<AxisOption> {
    let mut options = Vec::with_capacity(obj_names.len() + 5);

    // 目的関数オプション
    for (i, name) in obj_names.iter().enumerate() {
        options.push(AxisOption {
            id: format!("Objective{}", i),
            label: format!("Objective {} ({})", i, name),
        });
    }

    // MCDM方法別スコアオプション
    match mcdm_result {
        McdmResult::Vikor(_) => {
            for (id, label) in [
                (AXIS_VIKOR_Q, "VIKOR Q Score"),
                (AXIS_VIKOR_S, "VIKOR S Value"),
                (AXIS_VIKOR_R, "VIKOR R Value"),
            ] {
                options.push(AxisOption {
                    id: id.to_string(),
                    label: label.to_string(),
                });
            }
        }
        McdmResult::Topsis(_) => {
            options.push(AxisOption {
                id: AXIS_TOPSIS_SCORE.to_string(),
                label: "TOPSIS Score".to_string(),
            });
        }
        McdmResult::PrometheeI(_) | McdmResult::PrometheeII(_) => {
            for (id, label) in [
                (AXIS_PHI_PLUS, "Phi+ (Positive Flow)"),
                (AXIS_PHI_MINUS, "Phi- (Negative Flow)"),
                (AXIS_PHI_NET, "Phi Net"),
            ] {
                options.push(AxisOption {
                    id: id.to_string(),
                    label: label.to_string(),
                });
            }
        }
    }

    options
}

// ──────────────────────────────────────────────────────────────
// 軸値抽出
// ──────────────────────────────────────────────────────────────

/// 指定された軸識別子から各トライアルの値を抽出する
pub(crate) fn extract_axis_values(
    axis_id: &str,
    mcdm_result: &McdmResult,
    view: &StudyView,
    obj_names: &[String],
) -> Result<Vec<f64>, String> {
    // 目的関数 "Objective{N}" の場合
    if let Some(idx_str) = axis_id.strip_prefix("Objective") {
        let idx: usize = idx_str
            .parse()
            .map_err(|_| format!("Invalid objective index in axis: '{}'", axis_id))?;
        let obj_name = obj_names
            .get(idx)
            .ok_or_else(|| format!("Objective index {} out of range", idx))?;
        let values = view
            .numeric_column(obj_name)
            .map(|col| col.to_vec())
            .unwrap_or_else(|| vec![f64::NAN; view.row_count()]);
        return Ok(values);
    }

    // MCDM方法別スコア（view に依存しない）
    match mcdm_result {
        McdmResult::Vikor(r) => {
            if axis_id == AXIS_VIKOR_Q {
                Ok(r.q_values.clone())
            } else if axis_id == AXIS_VIKOR_S {
                Ok(r.s_values.clone())
            } else if axis_id == AXIS_VIKOR_R {
                Ok(r.r_values.clone())
            } else {
                Err(format!("Unknown axis '{}' for VIKOR result", axis_id))
            }
        }
        McdmResult::Topsis(r) => {
            if axis_id == AXIS_TOPSIS_SCORE {
                Ok(r.scores.clone())
            } else {
                Err(format!("Unknown axis '{}' for TOPSIS result", axis_id))
            }
        }
        McdmResult::PrometheeI(r) | McdmResult::PrometheeII(r) => {
            if axis_id == AXIS_PHI_PLUS {
                Ok(r.phi_plus.clone())
            } else if axis_id == AXIS_PHI_MINUS {
                Ok(r.phi_minus.clone())
            } else if axis_id == AXIS_PHI_NET {
                Ok(r.phi_net.clone())
            } else {
                Err(format!("Unknown axis '{}' for PROMETHEE result", axis_id))
            }
        }
    }
}

// ──────────────────────────────────────────────────────────────
// Min-Max 正規化
// ──────────────────────────────────────────────────────────────

/// Min-Max正規化 (v - min) / (max - min)
/// - 全値同一の場合は 0.5 を返す
/// - NaN/Inf は NaN のまま（呼び出し元でフィルタ）
pub(crate) fn normalize_values(values: &[f64]) -> Vec<f64> {
    if values.is_empty() {
        return vec![];
    }

    let mut min = f64::INFINITY;
    let mut max = f64::NEG_INFINITY;
    for &v in values {
        if v.is_finite() {
            if v < min {
                min = v;
            }
            if v > max {
                max = v;
            }
        }
    }
    if min == f64::INFINITY {
        return vec![0.5; values.len()];
    }

    let range = max - min;
    values
        .iter()
        .map(|&v| {
            if !v.is_finite() {
                f64::NAN
            } else if range < f64::EPSILON {
                0.5
            } else {
                (v - min) / range
            }
        })
        .collect()
}

// ──────────────────────────────────────────────────────────────
// ランキングベース色分けマッピング
// ──────────────────────────────────────────────────────────────

/// ランキング順位から表示色を決定する
/// - rank 0-4:   常に Red（Top5に常時含む）
/// - rank 5-9:   threshold >= Top10 の場合 Orange
/// - rank 10-19: threshold >= Top20 の場合 Yellow
/// - その他:     Gray
pub(crate) fn map_rank_to_color(rank: usize, threshold: ScatterTopN) -> Color32 {
    match rank {
        0..=4 => COLOR_MCDM_HIGH,
        5..=9 if threshold >= ScatterTopN::Top10 => COLOR_MCDM_MID,
        10..=19 if threshold >= ScatterTopN::Top20 => COLOR_MCDM_LOW,
        _ => COLOR_MCDM_NONE,
    }
}

/// trial_idx → rank の逆引きマップを構築する
/// ranked_indices[rank] = trial_idx なので逆引きが必要
fn build_rank_map(ranked_indices: &[u32], n_trials: usize) -> Vec<usize> {
    let mut rank_map = vec![usize::MAX; n_trials];
    for (rank, &trial_idx) in ranked_indices.iter().enumerate() {
        let idx = trial_idx as usize;
        if idx < n_trials {
            rank_map[idx] = rank;
        }
    }
    rank_map
}

// ──────────────────────────────────────────────────────────────
// 散布図ポイント計算
// ──────────────────────────────────────────────────────────────

/// 散布図の1点: (x座標, y座標, 色)。
type ScatterPoint = (f64, f64, Color32);

/// MCDM散布図ポイントを計算する
/// - 軸値抽出 → 色付け
/// - 戻り値: (実行可能解ポイント, 実行不可能解ポイント, メタデータ)
pub(crate) fn compute_scatter_points(
    mcdm_result: &McdmResult,
    view: &StudyView,
    obj_names: &[String],
    x_axis: &str,
    y_axis: &str,
    color_threshold: ScatterTopN,
) -> Result<(Vec<ScatterPoint>, Vec<(f64, f64)>, ScatterMetadata), String> {
    let n_trials = view.row_count();
    if n_trials == 0 {
        return Ok((
            vec![],
            vec![],
            ScatterMetadata {
                total_trials: 0,
                compute_time_ms: 0.0,
            },
        ));
    }

    let x_vals = extract_axis_values(x_axis, mcdm_result, view, obj_names)?;
    let y_vals = extract_axis_values(y_axis, mcdm_result, view, obj_names)?;
    let is_feasible_col = view.numeric_column("is_feasible");

    let ranked = mcdm_result.ranked_indices();
    let rank_map = build_rank_map(ranked, n_trials);

    let mut feasible_pts: Vec<ScatterPoint> = Vec::with_capacity(n_trials);
    let mut infeasible_pts: Vec<(f64, f64)> = Vec::new();

    for i in 0..n_trials {
        let x = match x_vals.get(i).copied() {
            Some(v) if v.is_finite() => v,
            _ => continue,
        };
        let y = match y_vals.get(i).copied() {
            Some(v) if v.is_finite() => v,
            _ => continue,
        };

        let feasible = is_feasible_col
            .and_then(|c| c.get(i))
            .map(|&v| v > 0.5)
            .unwrap_or(true);

        if !feasible {
            infeasible_pts.push((x, y));
            continue;
        }

        let rank = rank_map[i];
        let color = if rank == usize::MAX {
            COLOR_MCDM_NONE
        } else {
            map_rank_to_color(rank, color_threshold)
        };
        feasible_pts.push((x, y, color));
    }

    let total = feasible_pts.len() + infeasible_pts.len();
    Ok((
        feasible_pts,
        infeasible_pts,
        ScatterMetadata {
            total_trials: total,
            compute_time_ms: 0.0,
        },
    ))
}

// ──────────────────────────────────────────────────────────────
// ユニットテスト
// ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::results::{PrometheeResult, TopsisResult, VikorResult};
    use std::collections::HashMap;
    use std::sync::Arc;
    use tunny_core::dataframe::{DataFrame, TrialRow as CoreRow};

    // ── テストヘルパー ──────────────────────────────────────────

    fn make_view_with_objectives(objective_rows: &[Vec<f64>]) -> (StudyView, Vec<String>) {
        let n = objective_rows.len();
        if n == 0 {
            let df = DataFrame::from_trials(&[], &[], &[], &[], &[], 0);
            return (StudyView::new(Arc::new(df), vec![]), vec![]);
        }
        let n_obj = objective_rows[0].len();
        let obj_names: Vec<String> = (0..n_obj).map(|i| format!("obj{i}")).collect();
        let core_rows: Vec<CoreRow> = (0..n)
            .map(|i| CoreRow {
                trial_id: i as u32,
                param_display: HashMap::new(),
                param_category_label: HashMap::new(),
                objective_values: objective_rows[i].clone(),
                user_attrs_numeric: HashMap::new(),
                user_attrs_string: HashMap::new(),
                constraint_values: vec![],
            })
            .collect();
        let df = DataFrame::from_trials(&core_rows, &[], &obj_names, &[], &[], 0);
        (StudyView::new(Arc::new(df), vec![0; n]), obj_names)
    }

    fn make_empty_view() -> StudyView {
        let df = DataFrame::from_trials(&[], &[], &[], &[], &[], 0);
        StudyView::new(Arc::new(df), vec![])
    }

    fn make_vikor(n: usize) -> VikorResult {
        let values: Vec<f64> = (0..n).map(|i| i as f64 * 0.1).collect();
        VikorResult {
            s_values: values.clone(),
            r_values: values.clone(),
            q_values: values.clone(),
            display_scores: values.iter().map(|v| 1.0 - v).collect(),
            ranked_indices: (0..n as u32).collect(),
            best_values: vec![0.0; 2],
            worst_values: vec![1.0; 2],
            duration_ms: 1.0,
        }
    }

    fn make_vikor_result(n: usize) -> McdmResult {
        McdmResult::Vikor(make_vikor(n))
    }

    fn make_topsis(n: usize) -> TopsisResult {
        TopsisResult {
            scores: (0..n).map(|i| i as f64 / n as f64).collect(),
            ranked_indices: (0..n as u32).rev().collect(),
            positive_ideal: vec![],
            negative_ideal: vec![],
            duration_ms: 1.0,
        }
    }

    fn make_promethee(n: usize) -> PrometheeResult {
        let v: Vec<f64> = (0..n).map(|i| i as f64 * 0.05).collect();
        PrometheeResult {
            phi_plus: v.clone(),
            phi_minus: v.iter().map(|x| 1.0 - x).collect(),
            phi_net: v.clone(),
            ranked_indices_i: (0..n as u32).collect(),
            ranked_indices_ii: (0..n as u32).collect(),
            duration_ms: 1.0,
        }
    }

    // ── 構造体・初期化テスト ─────────────────────────────────────

    #[test]
    fn test_scatter_chart_new_defaults() {
        let chart = McdmScatterChart::new();
        assert_eq!(chart.x_axis, "Objective0");
        assert_eq!(chart.y_axis, "Objective1");
        assert_eq!(chart.color_threshold, ScatterTopN::Top10);
        assert!(chart.display_rows_cache.is_none());
        assert!(chart.cache_key.is_none());
        assert!(chart.error_message.is_none());
    }

    #[test]
    fn test_invalidate_cache_clears_data() {
        let mut chart = McdmScatterChart::new();
        chart.display_rows_cache = Some(vec![(0.5, 0.5, Color32::RED)]);
        chart.error_message = Some("error".to_string());
        chart.cache_key = Some(chart.make_cache_key(10, &McdmResult::Topsis(make_topsis(10))));
        chart.invalidate_cache();
        assert!(chart.display_rows_cache.is_none());
        assert!(chart.cache_key.is_none());
        assert!(chart.error_message.is_none());
    }

    #[test]
    fn test_cache_stale_when_no_key() {
        let chart = McdmScatterChart::new();
        assert!(chart.is_cache_stale(100, &McdmResult::Topsis(make_topsis(100))));
    }

    #[test]
    fn test_cache_stale_when_trial_count_changes() {
        let mut chart = McdmScatterChart::new();
        let result = McdmResult::Topsis(make_topsis(100));
        chart.cache_key = Some(chart.make_cache_key(100, &result));
        assert!(chart.is_cache_stale(150, &result)); // 150 ≠ 100
    }

    #[test]
    fn test_cache_not_stale_same_key() {
        let mut chart = McdmScatterChart::new();
        let result = McdmResult::Topsis(make_topsis(100));
        chart.cache_key = Some(chart.make_cache_key(100, &result));
        assert!(!chart.is_cache_stale(100, &result));
    }

    // ── get_axis_options テスト ──────────────────────────────────

    #[test]
    fn test_axis_options_vikor_has_scores() {
        let result = McdmResult::Vikor(make_vikor(3));
        let obj_names = vec!["obj0".to_string(), "obj1".to_string()];
        let options = get_axis_options(&result, &obj_names);

        assert!(options.iter().any(|o| o.id == "Objective0"));
        assert!(options.iter().any(|o| o.id == "Objective1"));
        assert!(options.iter().any(|o| o.id == "VIKOR_Q"));
        assert!(options.iter().any(|o| o.id == "VIKOR_S"));
        assert!(options.iter().any(|o| o.id == "VIKOR_R"));
    }

    #[test]
    fn test_axis_options_topsis() {
        let result = McdmResult::Topsis(make_topsis(3));
        let options = get_axis_options(&result, &["obj".to_string()]);
        assert!(options.iter().any(|o| o.id == "TOPSIS_Score"));
        assert!(!options.iter().any(|o| o.id == "VIKOR_Q"));
    }

    #[test]
    fn test_axis_options_promethee() {
        let result = McdmResult::PrometheeI(make_promethee(3));
        let options = get_axis_options(&result, &[]);
        assert!(options.iter().any(|o| o.id == "Phi+"));
        assert!(options.iter().any(|o| o.id == "Phi-"));
        assert!(options.iter().any(|o| o.id == "Phi_Net"));
    }

    #[test]
    fn test_axis_options_empty_objectives() {
        let result = McdmResult::Topsis(make_topsis(3));
        let options = get_axis_options(&result, &[]);
        // TOPSIS_Score だけ
        assert_eq!(options.len(), 1);
        assert_eq!(options[0].id, "TOPSIS_Score");
    }

    // ── extract_axis_values テスト ────────────────────────────────

    #[test]
    fn test_extract_objective0() {
        let (view, obj_names) = make_view_with_objectives(&[vec![1.0, 2.0], vec![3.0, 4.0]]);
        let result = McdmResult::Vikor(make_vikor(2));
        let vals = extract_axis_values("Objective0", &result, &view, &obj_names).unwrap();
        assert_eq!(vals, vec![1.0, 3.0]);
    }

    #[test]
    fn test_extract_objective1() {
        let (view, obj_names) = make_view_with_objectives(&[vec![1.0, 2.0], vec![3.0, 4.0]]);
        let result = McdmResult::Vikor(make_vikor(2));
        let vals = extract_axis_values("Objective1", &result, &view, &obj_names).unwrap();
        assert_eq!(vals, vec![2.0, 4.0]);
    }

    #[test]
    fn test_extract_vikor_q() {
        let vikor = make_vikor(3);
        let q = vikor.q_values.clone();
        let result = McdmResult::Vikor(vikor);
        let view = make_empty_view();
        let vals = extract_axis_values("VIKOR_Q", &result, &view, &[]).unwrap();
        assert_eq!(vals, q);
    }

    #[test]
    fn test_extract_topsis_score() {
        let topsis = make_topsis(3);
        let scores = topsis.scores.clone();
        let result = McdmResult::Topsis(topsis);
        let view = make_empty_view();
        let vals = extract_axis_values("TOPSIS_Score", &result, &view, &[]).unwrap();
        assert_eq!(vals, scores);
    }

    #[test]
    fn test_extract_phi_plus() {
        let promethee = make_promethee(3);
        let phi_plus = promethee.phi_plus.clone();
        let result = McdmResult::PrometheeI(promethee);
        let view = make_empty_view();
        let vals = extract_axis_values("Phi+", &result, &view, &[]).unwrap();
        assert_eq!(vals, phi_plus);
    }

    #[test]
    fn test_extract_unknown_axis_error() {
        let result = McdmResult::Vikor(make_vikor(3));
        let view = make_empty_view();
        let err = extract_axis_values("NonExistent", &result, &view, &[]);
        assert!(err.is_err());
    }

    #[test]
    fn test_extract_out_of_range_objective() {
        let (view, obj_names) = make_view_with_objectives(&[vec![1.0]]);
        let result = McdmResult::Vikor(make_vikor(1));
        // obj_names は ["obj0"] のみ。Objective5 は out of range → エラー
        let err = extract_axis_values("Objective5", &result, &view, &obj_names);
        assert!(err.is_err());
    }

    // ── normalize_values テスト ──────────────────────────────────

    #[test]
    fn test_normalize_basic() {
        let v = vec![100.0, 150.0, 200.0];
        let n = normalize_values(&v);
        assert!((n[0] - 0.0).abs() < 1e-10);
        assert!((n[1] - 0.5).abs() < 1e-10);
        assert!((n[2] - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_normalize_all_equal_gives_half() {
        let v = vec![5.0, 5.0, 5.0];
        let n = normalize_values(&v);
        assert!(n.iter().all(|&x| (x - 0.5).abs() < 1e-10));
    }

    #[test]
    fn test_normalize_empty() {
        assert!(normalize_values(&[]).is_empty());
    }

    #[test]
    fn test_normalize_negative_values() {
        let v = vec![-100.0, -50.0, 0.0];
        let n = normalize_values(&v);
        assert!((n[0] - 0.0).abs() < 1e-10);
        assert!((n[1] - 0.5).abs() < 1e-10);
        assert!((n[2] - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_normalize_with_nan_stays_nan() {
        let v = vec![1.0, f64::NAN, 3.0];
        let n = normalize_values(&v);
        assert!((n[0] - 0.0).abs() < 1e-10);
        assert!(n[1].is_nan());
        assert!((n[2] - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_normalize_single_value() {
        let v = vec![42.0];
        let n = normalize_values(&v);
        assert!((n[0] - 0.5).abs() < 1e-10);
    }

    // ── map_rank_to_color テスト ─────────────────────────────────

    #[test]
    fn test_rank_0_always_red() {
        for t in ScatterTopN::all() {
            assert_eq!(map_rank_to_color(0, *t), COLOR_MCDM_HIGH);
        }
    }

    #[test]
    fn test_rank_4_always_red() {
        for t in ScatterTopN::all() {
            assert_eq!(map_rank_to_color(4, *t), COLOR_MCDM_HIGH);
        }
    }

    #[test]
    fn test_rank_5_gray_when_top5() {
        assert_eq!(map_rank_to_color(5, ScatterTopN::Top5), COLOR_MCDM_NONE);
    }

    #[test]
    fn test_rank_5_orange_when_top10() {
        assert_eq!(map_rank_to_color(5, ScatterTopN::Top10), COLOR_MCDM_MID);
    }

    #[test]
    fn test_rank_5_orange_when_top20() {
        assert_eq!(map_rank_to_color(5, ScatterTopN::Top20), COLOR_MCDM_MID);
    }

    #[test]
    fn test_rank_10_gray_when_top5() {
        assert_eq!(map_rank_to_color(10, ScatterTopN::Top5), COLOR_MCDM_NONE);
    }

    #[test]
    fn test_rank_10_gray_when_top10() {
        assert_eq!(map_rank_to_color(10, ScatterTopN::Top10), COLOR_MCDM_NONE);
    }

    #[test]
    fn test_rank_10_yellow_when_top20() {
        assert_eq!(map_rank_to_color(10, ScatterTopN::Top20), COLOR_MCDM_LOW);
    }

    #[test]
    fn test_rank_50_always_gray() {
        for t in ScatterTopN::all() {
            assert_eq!(map_rank_to_color(50, *t), COLOR_MCDM_NONE);
        }
    }

    #[test]
    fn test_scatter_top_n_ordering() {
        assert!(ScatterTopN::Top5 < ScatterTopN::Top10);
        assert!(ScatterTopN::Top10 < ScatterTopN::Top20);
        assert!(ScatterTopN::Top5 < ScatterTopN::Top20);
    }

    // ── build_rank_map テスト ────────────────────────────────────

    #[test]
    fn test_build_rank_map_basic() {
        let ranked: Vec<u32> = vec![5, 2, 8];
        let map = build_rank_map(&ranked, 10);
        assert_eq!(map[5], 0);
        assert_eq!(map[2], 1);
        assert_eq!(map[8], 2);
        assert_eq!(map[0], usize::MAX); // ランク外
        assert_eq!(map[3], usize::MAX);
    }

    #[test]
    fn test_build_rank_map_all_trials() {
        let n = 5usize;
        let ranked: Vec<u32> = vec![4, 3, 2, 1, 0];
        let map = build_rank_map(&ranked, n);
        assert_eq!(map[4], 0); // trial 4 が rank 0（最良）
        assert_eq!(map[0], 4); // trial 0 が rank 4（最悪）
    }

    // ── compute_scatter_points 統合テスト ─────────────────────────

    #[test]
    fn test_compute_scatter_points_basic() {
        let n = 10;
        let data: Vec<Vec<f64>> = (0..n).map(|i| vec![i as f64, (n - i) as f64]).collect();
        let (view, obj_names) = make_view_with_objectives(&data);
        let result = make_vikor_result(n);

        let (points, _, meta) = compute_scatter_points(
            &result,
            &view,
            &obj_names,
            "Objective0",
            "Objective1",
            ScatterTopN::Top10,
        )
        .unwrap();

        assert_eq!(points.len(), n);
        assert_eq!(meta.total_trials, n);
        // Objective0 → [0,1,...,9]: trial 0 の生値 = 0.0
        assert!((points[0].0 - 0.0).abs() < 1e-10);
        // Objective1 for trial 0 = 10 (n - 0): 生値そのまま
        assert!((points[0].1 - 10.0).abs() < 1e-10);
    }

    #[test]
    fn test_compute_scatter_points_top5_rank0_is_red() {
        let n = 20;
        let data: Vec<Vec<f64>> = (0..n).map(|i| vec![i as f64, i as f64]).collect();
        let (view, obj_names) = make_view_with_objectives(&data);
        let result = make_vikor_result(n);

        let (points, _, _) = compute_scatter_points(
            &result,
            &view,
            &obj_names,
            "Objective0",
            "Objective1",
            ScatterTopN::Top5,
        )
        .unwrap();

        // Trial 0 is rank 0 (ranked_indices[0] = 0), should be Red
        assert_eq!(points[0].2, COLOR_MCDM_HIGH);
    }

    #[test]
    fn test_compute_scatter_points_empty_trials() {
        let vikor = make_vikor(0);
        let result = McdmResult::Vikor(vikor);
        let view = make_empty_view();
        let (points, _, meta) = compute_scatter_points(
            &result,
            &view,
            &[],
            "Objective0",
            "Objective1",
            ScatterTopN::Top10,
        )
        .unwrap();
        assert!(points.is_empty());
        assert_eq!(meta.total_trials, 0);
    }

    #[test]
    fn test_compute_scatter_points_vikor_axis() {
        let n = 5;
        let data: Vec<Vec<f64>> = (0..n).map(|i| vec![i as f64]).collect();
        let (view, obj_names) = make_view_with_objectives(&data);
        let result = make_vikor_result(n);

        let (points, _, _) = compute_scatter_points(
            &result,
            &view,
            &obj_names,
            "VIKOR_Q",
            "VIKOR_S",
            ScatterTopN::Top10,
        )
        .unwrap();

        // q_values == s_values for make_vikor, both are raw i * 0.1 values
        assert_eq!(points.len(), n);
    }
}
