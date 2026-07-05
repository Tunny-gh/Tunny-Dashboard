//! MCDM Scatter Chart Widget

use std::collections::HashMap;

use crate::io::artifacts::ArtifactEntry;
use crate::state::results::{McdmMethod, McdmResult};
use crate::state::types::{ColormapName, StudyView};
use crate::theme::chart_colors::{COLOR_EMPTY_STATE, COLOR_MCDM_NONE, COLOR_UNSELECTED_POINT};
use crate::theme::color_compute::compute_point_alpha;
use crate::theme::colormap::ColorMap;
use crate::theme::ERROR_COLOR;
use crate::ui::widgets::common::plot_nav::{apply_wheel_zoom, UnifiedNav};
use crate::ui::widgets::mcdm_chart::McdmControls;
use crate::ui::widgets::trial_detail_modal::{
    hit_test_nearest, TrialDetailModal, TrialDetailTarget, HIT_THRESHOLD,
};
use egui::Color32;

/// 軸識別子定数（get_axis_options と extract_axis_values で共有）
const AXIS_VIKOR_Q: &str = "VIKOR_Q";
const AXIS_VIKOR_S: &str = "VIKOR_S";
const AXIS_VIKOR_R: &str = "VIKOR_R";
const AXIS_TOPSIS_SCORE: &str = "TOPSIS_Score";
const AXIS_PHI_PLUS: &str = "Phi+";
const AXIS_PHI_MINUS: &str = "Phi-";
const AXIS_PHI_NET: &str = "Phi_Net";

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
    colormap_name: ColormapName,
    top_n: usize,
    /// MCDM手法（手法切替を検知）
    result_method: McdmMethod,
    /// 先頭スコアのビット表現（重み変更を検知）
    result_score0_bits: u64,
    /// スコア件数
    result_score_count: usize,
}

/// MCDM 散布図ウィジェット
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct McdmScatterChart {
    /// MCDM 設定・実行状態（手法 / 重み / Run など）
    pub controls: McdmControls,
    /// X軸の軸識別子
    pub x_axis: String,
    /// Y軸の軸識別子
    pub y_axis: String,
    /// 点クリックで開くトライアル詳細モーダル。
    #[serde(skip)]
    pub detail_modal: TrialDetailModal,
    // --- 内部キャッシュ状態 ---
    #[serde(skip)]
    display_rows_cache: Option<Vec<(f64, f64, Color32, u32)>>,
    #[serde(skip)]
    infeasible_cache: Option<Vec<(f64, f64)>>,
    /// 点クリック判定用の候補（trial_id, 行 index, 座標）。display_rows_cache と同じキーで更新する。
    #[serde(skip)]
    hit_candidates: Option<Vec<(u32, usize, [f64; 2])>>,
    #[serde(skip)]
    metadata: Option<ScatterMetadata>,
    #[serde(skip)]
    error_message: Option<String>,
    #[serde(skip)]
    cache_key: Option<CacheKey>,
}

impl Default for McdmScatterChart {
    fn default() -> Self {
        Self {
            controls: McdmControls::default(),
            x_axis: "Objective0".to_string(),
            y_axis: "Objective1".to_string(),
            detail_modal: TrialDetailModal::new(),
            display_rows_cache: None,
            infeasible_cache: None,
            hit_candidates: None,
            metadata: None,
            error_message: None,
            cache_key: None,
        }
    }
}

impl McdmScatterChart {
    /// グローバル widget の MCDM 実行状態を取り込む（キャンバスの各アイテム用）。
    pub fn adopt_compute_state(&mut self, src: &Self) {
        self.controls.adopt_compute_state(&src.controls);
    }

    /// 現在の設定からキャッシュキーを生成する
    fn make_cache_key(
        &self,
        trial_count: usize,
        result: &McdmResult,
        colormap_name: &ColormapName,
        top_n: usize,
    ) -> CacheKey {
        let scores = result.primary_scores();
        CacheKey {
            trial_count,
            x_axis: self.x_axis.clone(),
            y_axis: self.y_axis.clone(),
            colormap_name: colormap_name.clone(),
            top_n,
            result_method: result.method(),
            result_score0_bits: scores.first().copied().unwrap_or(0.0).to_bits(),
            result_score_count: scores.len(),
        }
    }

    /// キャッシュが陳腐化しているか確認する
    fn is_cache_stale(
        &self,
        trial_count: usize,
        result: &McdmResult,
        colormap_name: &ColormapName,
        top_n: usize,
    ) -> bool {
        let scores = result.primary_scores();
        let score0_bits = scores.first().copied().unwrap_or(0.0).to_bits();
        match &self.cache_key {
            None => true,
            Some(key) => {
                key.trial_count != trial_count
                    || key.x_axis != self.x_axis
                    || key.y_axis != self.y_axis
                    || key.colormap_name != *colormap_name
                    || key.top_n != top_n
                    || key.result_method != result.method()
                    || key.result_score0_bits != score0_bits
                    || key.result_score_count != scores.len()
            }
        }
    }

    /// ウィジェットを描画する
    #[allow(clippy::too_many_arguments)]
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        mcdm_result: Option<&McdmResult>,
        view: &StudyView,
        param_names: &[String],
        obj_names: &[String],
        colormap: &ColorMap,
        colormap_name: &ColormapName,
        artifact_map: &HashMap<u32, Vec<ArtifactEntry>>,
        selected_indices: &[u32],
    ) {
        if !self.controls.show_controls(ui, obj_names, "mcdm_scatter") {
            return;
        }
        if self.controls.computing {
            return;
        }
        let top_n = self.controls.top_n.value();

        let Some(result) = mcdm_result else {
            ui.centered_and_justified(|ui| {
                ui.colored_label(COLOR_EMPTY_STATE, "Press Run to compute the MCDM ranking");
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
        });

        let n_trials = view.row_count();

        // キャッシュが陳腐化している場合に再計算
        if self.is_cache_stale(n_trials, result, colormap_name, top_n) {
            let new_key = self.make_cache_key(n_trials, result, colormap_name, top_n);
            let start = std::time::Instant::now();

            match compute_scatter_points(
                result,
                view,
                obj_names,
                &self.x_axis,
                &self.y_axis,
                colormap,
                top_n,
            ) {
                Ok((points, infeasible, mut meta)) => {
                    meta.compute_time_ms = start.elapsed().as_secs_f64() * 1000.0;
                    self.display_rows_cache = Some(points);
                    self.infeasible_cache = Some(infeasible);
                    self.hit_candidates = Some(compute_hit_candidates(
                        result,
                        view,
                        obj_names,
                        &self.x_axis,
                        &self.y_axis,
                    ));
                    self.cache_key = Some(new_key);
                    self.metadata = Some(meta);
                    self.error_message = None;
                }
                Err(e) => {
                    self.error_message = Some(e);
                    self.display_rows_cache = None;
                    self.infeasible_cache = None;
                    self.hit_candidates = None;
                    self.cache_key = None;
                }
            }
        }

        if let Some(ref error) = self.error_message {
            ui.colored_label(ERROR_COLOR, error);
            return;
        }

        let empty = vec![];
        let infeasible = self.infeasible_cache.as_deref().unwrap_or(&empty);
        let no_candidates = vec![];
        let candidates = self.hit_candidates.as_deref().unwrap_or(&no_candidates);
        let mut clicked_detail: Option<(u32, usize)> = None;
        if let Some(ref points) = self.display_rows_cache {
            clicked_detail = render_scatter_plot(
                ui,
                points,
                infeasible,
                candidates,
                &self.x_axis,
                &self.y_axis,
                colormap,
                top_n,
                selected_indices,
            );
        }

        // 点クリックでトライアル詳細モーダルを開く（散布図情報 = MCDM ランク・スコア）。
        if let Some((trial_id, row)) = clicked_detail {
            let rank_map = build_rank_map(result.ranked_indices(), view.row_count());
            let rank = rank_map.get(row).copied().unwrap_or(usize::MAX);
            let rank_str = if rank == usize::MAX {
                "—".to_string()
            } else {
                (rank + 1).to_string()
            };
            let score = result.primary_scores().get(row).copied();
            let mut context = vec![("MCDM Rank".to_string(), rank_str)];
            context.push((
                "Score".to_string(),
                score
                    .map(|s| format!("{s:.4}"))
                    .unwrap_or_else(|| "—".to_string()),
            ));
            // VIKOR: 妥協解集合（C1/C2）に属する点はモーダルでも明示する。
            if let McdmResult::Vikor(vr) = result {
                if vr.compromise_indices.contains(&row) {
                    context.push(("VIKOR Compromise".to_string(), "★ Yes (C1/C2)".to_string()));
                }
            }
            self.detail_modal.open(TrialDetailTarget {
                trial_id,
                row_index: row,
                context,
            });
        }
        self.detail_modal
            .show(ui, view, param_names, obj_names, artifact_map);

        ui.separator();
        // 選択フィルタ中は、スコアがフロント全体基準である旨を明示する。
        if !selected_indices.is_empty() {
            ui.label(
                egui::RichText::new(
                    "Highlighting selection. Scores are computed over the full Pareto front.",
                )
                .small()
                .weak(),
            );
        }
        // VIKOR: 妥協解集合（Opricovic & Tzeng の受容条件 C1/C2 を満たす解）を明示する。
        // C1 が不成立の場合は複数解になるため、trial 番号のリストで表示する。
        if let McdmResult::Vikor(vr) = result {
            if !vr.compromise_indices.is_empty() {
                let labels: Vec<String> = vr
                    .compromise_indices
                    .iter()
                    .map(|&row| {
                        view.df
                            .get_trial_number(row)
                            .map(|n| format!("#{n}"))
                            .unwrap_or_else(|| format!("row {row}"))
                    })
                    .collect();
                ui.label(
                    egui::RichText::new(format!(
                        "★ VIKOR compromise set (C1/C2): {}",
                        labels.join(", ")
                    ))
                    .small()
                    .strong(),
                );
            }
        }
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

/// 散布図を描画し、点がクリックされた場合は `(trial_id, 行 index)` を返す。
#[allow(clippy::too_many_arguments)]
fn render_scatter_plot(
    ui: &mut egui::Ui,
    points: &[(f64, f64, Color32, u32)],
    infeasible: &[(f64, f64)],
    hit_candidates: &[(u32, usize, [f64; 2])],
    x_label: &str,
    y_label: &str,
    colormap: &ColorMap,
    top_n: usize,
    selected_indices: &[u32],
) -> Option<(u32, usize)> {
    use std::collections::HashMap;

    // 未ランク（COLOR_MCDM_NONE）とランク済みを分離。
    // 選択フィルタ（PCP ブラシ等）が有効な場合、選択外は淡色にまとめて背面に描く。
    // スコア・色はフロント全体基準のまま。ここでの分岐は表示上の強調に限る。
    let mut none_pts: Vec<[f64; 2]> = Vec::new();
    let mut dim_pts: Vec<[f64; 2]> = Vec::new();
    // 色 → 座標リスト（輝度でソートするため u32 輝度値も保持）
    let mut color_groups: HashMap<[u8; 4], (Vec<[f64; 2]>, u32)> = HashMap::new();

    for &(x, y, color, trial_id) in points {
        if compute_point_alpha(trial_id, selected_indices) != 255 {
            dim_pts.push([x, y]);
            continue;
        }
        if color == COLOR_MCDM_NONE {
            none_pts.push([x, y]);
        } else {
            let key = [color.r(), color.g(), color.b(), color.a()];
            let lum = color.r() as u32 + color.g() as u32 + color.b() as u32;
            let entry = color_groups.entry(key).or_insert((Vec::new(), lum));
            entry.0.push([x, y]);
        }
    }

    // 輝度順にソート（暗い順→明るい順で手前に描画）
    let mut sorted: Vec<_> = color_groups.into_iter().collect();
    sorted.sort_by_key(|(_, (_, lum))| *lum);

    // 判例用の代表色
    let best_color = colormap.interpolate(1.0);
    let worst_color = if top_n > 1 {
        colormap.interpolate(0.0)
    } else {
        best_color
    };
    // 凡例から表示/非表示を切り替えられるため、常に描画する
    let has_infeasible = !infeasible.is_empty();

    let mut clicked_detail: Option<(u32, usize)> = None;
    egui_plot::Plot::new("mcdm_scatter_plot")
        .unified_nav()
        .x_axis_label(x_label)
        .y_axis_label(y_label)
        .legend(egui_plot::Legend::default())
        .show(ui, |plot_ui| {
            apply_wheel_zoom(plot_ui);
            // 点クリックで詳細モーダルを開く対象を検出する。
            let resp = plot_ui.response();
            if resp.clicked_by(egui::PointerButton::Primary) {
                clicked_detail = resp
                    .interact_pointer_pos()
                    .and_then(|pos| hit_test_nearest(plot_ui, hit_candidates, pos, HIT_THRESHOLD));
            }
            // 実行不可能解を最背面に描画
            if has_infeasible {
                let pts: Vec<[f64; 2]> = infeasible.iter().map(|&(x, y)| [x, y]).collect();
                plot_ui.points(
                    egui_plot::Points::new("Infeasible", pts)
                        .color(crate::theme::chart_colors::COLOR_INFEASIBLE)
                        .radius(3.0),
                );
            }
            // 選択フィルタ外（灰色・最背面、凡例は "Others (unselected)" に集約）
            if !dim_pts.is_empty() {
                plot_ui.points(
                    egui_plot::Points::new("Others (unselected)", dim_pts)
                        .color(COLOR_UNSELECTED_POINT)
                        .radius(2.5),
                );
            }
            // 未ランク（グレー）
            if !none_pts.is_empty() {
                plot_ui.points(
                    egui_plot::Points::new("Others", none_pts)
                        .color(COLOR_MCDM_NONE)
                        .radius(3.0),
                );
            }
            // ランク済み：暗い（下位）→明るい（上位）の順
            for ([r, g, b, a], (pts, _)) in sorted {
                let color = Color32::from_rgba_unmultiplied(r, g, b, a);
                plot_ui.points(egui_plot::Points::new("", pts).color(color).radius(4.0));
            }
            // 判例専用エントリ（データなし・名前のみ）
            plot_ui.points(
                egui_plot::Points::new("Rank 1 (Best)", Vec::<[f64; 2]>::new())
                    .color(best_color)
                    .radius(5.0),
            );
            if top_n > 1 {
                plot_ui.points(
                    egui_plot::Points::new(format!("Rank {top_n}"), Vec::<[f64; 2]>::new())
                        .color(worst_color)
                        .radius(5.0),
                );
            }
        });
    clicked_detail
}

/// クリック判定用の候補（trial_id, 行 index, 座標）を計算する。
/// 散布図に描画される有限値の点のみを対象にする（feasible / infeasible を問わない）。
fn compute_hit_candidates(
    mcdm_result: &McdmResult,
    view: &StudyView,
    obj_names: &[String],
    x_axis: &str,
    y_axis: &str,
) -> Vec<(u32, usize, [f64; 2])> {
    let (Ok(x_vals), Ok(y_vals)) = (
        extract_axis_values(x_axis, mcdm_result, view, obj_names),
        extract_axis_values(y_axis, mcdm_result, view, obj_names),
    ) else {
        return Vec::new();
    };
    (0..view.row_count())
        .filter_map(|i| {
            let x = x_vals.get(i).copied()?;
            let y = y_vals.get(i).copied()?;
            if !x.is_finite() || !y.is_finite() {
                return None;
            }
            let trial_id = view.trial_ids.get(i).copied().unwrap_or(i as u32);
            Some((trial_id, i, [x, y]))
        })
        .collect()
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

/// 散布図の1点: (x座標, y座標, 色, trial_id)。
/// trial_id は選択フィルタ（PCP ブラシ等）でのグレーアウト判定に使う。
type ScatterPoint = (f64, f64, Color32, u32);
/// `compute_scatter_points` の戻り値型エイリアス。
type ScatterPointsResult = (Vec<ScatterPoint>, Vec<(f64, f64)>, ScatterMetadata);

/// MCDM散布図ポイントを計算する
/// - 軸値抽出 → カラーマップによる連続着色
/// - 戻り値: (実行可能解ポイント, 実行不可能解ポイント, メタデータ)
pub(crate) fn compute_scatter_points(
    mcdm_result: &McdmResult,
    view: &StudyView,
    obj_names: &[String],
    x_axis: &str,
    y_axis: &str,
    colormap: &ColorMap,
    top_n: usize,
) -> Result<ScatterPointsResult, String> {
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
    let feas = view.feasibility();

    let ranked = mcdm_result.ranked_indices();
    let rank_map = build_rank_map(ranked, n_trials);
    // top_n の範囲でカラーコンターを割り当て、最低1は確保する
    let colored_range = top_n.max(1);

    let mut feasible_pts: Vec<ScatterPoint> = Vec::with_capacity(n_trials);
    let mut infeasible_pts: Vec<(f64, f64)> = Vec::new();

    for (i, &rank) in rank_map.iter().enumerate() {
        let x = match x_vals.get(i).copied() {
            Some(v) if v.is_finite() => v,
            _ => continue,
        };
        let y = match y_vals.get(i).copied() {
            Some(v) if v.is_finite() => v,
            _ => continue,
        };

        if !feas.is_feasible(i) {
            infeasible_pts.push((x, y));
            continue;
        }
        let color = if rank == usize::MAX || rank >= colored_range {
            // ランク外または top_n 外は灰色
            COLOR_MCDM_NONE
        } else {
            // rank 0（最良）→ t=1.0、rank colored_range-1 → t=0.0
            let t = if colored_range > 1 {
                1.0 - rank as f32 / (colored_range - 1) as f32
            } else {
                1.0
            };
            colormap.interpolate(t)
        };
        let trial_id = view.trial_ids.get(i).copied().unwrap_or(i as u32);
        feasible_pts.push((x, y, color, trial_id));
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
                trial_number: i as u32,
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
            compromise_indices: if n > 0 { vec![0] } else { vec![] },
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
            incomparable_counts: vec![0; n],
            duration_ms: 1.0,
        }
    }

    // ── 構造体・初期化テスト ─────────────────────────────────────

    #[test]
    fn test_scatter_chart_default_values() {
        let chart = McdmScatterChart::default();
        assert_eq!(chart.x_axis, "Objective0");
        assert_eq!(chart.y_axis, "Objective1");
        assert!(chart.display_rows_cache.is_none());
        assert!(chart.cache_key.is_none());
        assert!(chart.error_message.is_none());
    }

    #[test]
    fn test_cache_stale_when_no_key() {
        use crate::state::types::ColormapName;
        let chart = McdmScatterChart::default();
        assert!(chart.is_cache_stale(
            100,
            &McdmResult::Topsis(make_topsis(100)),
            &ColormapName::Viridis,
            10
        ));
    }

    #[test]
    fn test_cache_stale_when_trial_count_changes() {
        use crate::state::types::ColormapName;
        let cmap_name = ColormapName::Viridis;
        let mut chart = McdmScatterChart::default();
        let result = McdmResult::Topsis(make_topsis(100));
        chart.cache_key = Some(chart.make_cache_key(100, &result, &cmap_name, 10));
        assert!(chart.is_cache_stale(150, &result, &cmap_name, 10)); // 150 ≠ 100
    }

    #[test]
    fn test_cache_not_stale_same_key() {
        use crate::state::types::ColormapName;
        let cmap_name = ColormapName::Viridis;
        let mut chart = McdmScatterChart::default();
        let result = McdmResult::Topsis(make_topsis(100));
        chart.cache_key = Some(chart.make_cache_key(100, &result, &cmap_name, 10));
        assert!(!chart.is_cache_stale(100, &result, &cmap_name, 10));
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
        use crate::state::types::ColormapName;
        use crate::theme::colormap_name::colormap_from_name;
        let n = 10;
        let data: Vec<Vec<f64>> = (0..n).map(|i| vec![i as f64, (n - i) as f64]).collect();
        let (view, obj_names) = make_view_with_objectives(&data);
        let result = make_vikor_result(n);
        let cmap = colormap_from_name(&ColormapName::Viridis);

        let (points, _, meta) = compute_scatter_points(
            &result,
            &view,
            &obj_names,
            "Objective0",
            "Objective1",
            &cmap,
            n,
        )
        .unwrap();

        assert_eq!(points.len(), n);
        assert_eq!(meta.total_trials, n);
        assert!((points[0].0 - 0.0).abs() < 1e-10);
        assert!((points[0].1 - 10.0).abs() < 1e-10);
    }

    #[test]
    fn test_compute_scatter_points_rank0_gets_best_color() {
        use crate::state::types::ColormapName;
        use crate::theme::colormap_name::colormap_from_name;
        let n = 20;
        let top_n = 10_usize;
        let data: Vec<Vec<f64>> = (0..n).map(|i| vec![i as f64, i as f64]).collect();
        let (view, obj_names) = make_view_with_objectives(&data);
        let result = make_vikor_result(n);
        let cmap = colormap_from_name(&ColormapName::Viridis);

        let (points, _, _) = compute_scatter_points(
            &result,
            &view,
            &obj_names,
            "Objective0",
            "Objective1",
            &cmap,
            top_n,
        )
        .unwrap();

        // rank 0（best）→ t=1.0 → colormap の最高端
        let expected = cmap.interpolate(1.0);
        assert_eq!(points[0].2, expected);
        // top_n 外（rank >= top_n）は gray
        assert_eq!(points[n - 1].2, COLOR_MCDM_NONE);
    }

    #[test]
    fn test_compute_scatter_points_empty_trials() {
        use crate::state::types::ColormapName;
        use crate::theme::colormap_name::colormap_from_name;
        let vikor = make_vikor(0);
        let result = McdmResult::Vikor(vikor);
        let view = make_empty_view();
        let cmap = colormap_from_name(&ColormapName::Viridis);

        let (points, _, meta) =
            compute_scatter_points(&result, &view, &[], "Objective0", "Objective1", &cmap, 10)
                .unwrap();
        assert!(points.is_empty());
        assert_eq!(meta.total_trials, 0);
    }

    #[test]
    fn test_compute_scatter_points_vikor_axis() {
        use crate::state::types::ColormapName;
        use crate::theme::colormap_name::colormap_from_name;
        let n = 5;
        let data: Vec<Vec<f64>> = (0..n).map(|i| vec![i as f64]).collect();
        let (view, obj_names) = make_view_with_objectives(&data);
        let result = make_vikor_result(n);
        let cmap = colormap_from_name(&ColormapName::Viridis);

        let (points, _, _) =
            compute_scatter_points(&result, &view, &obj_names, "VIKOR_Q", "VIKOR_S", &cmap, n)
                .unwrap();

        assert_eq!(points.len(), n);
    }
}
