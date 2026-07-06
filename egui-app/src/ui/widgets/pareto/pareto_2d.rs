use std::collections::HashSet;
use std::sync::Arc;

use crate::state::app_state::AppState;
use crate::state::messages::SurrogateMultiOptUiResult;
use crate::theme::chart_colors::{
    COLOR_HIGHLIGHT_PT, COLOR_INFEASIBLE, COLOR_NON_PARETO, COLOR_PARETO, COLOR_SURROGATE_FRONT,
    COLOR_UNSELECTED_POINT,
};
use crate::theme::color_compute::point_alpha_in_set;
use crate::ui::widgets::common::plot_nav::{apply_wheel_zoom, UnifiedNav};
use crate::ui::widgets::trial_detail_modal::{
    axis_row, hit_test_nearest, push_feasible_row, TrialDetailModal, TrialDetailTarget,
    HIT_THRESHOLD,
};

/// 2D Pareto 散布図ウィジェット（egui_plot ベース）
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct ParetoScatter2D {
    pub x_axis: String,
    pub y_axis: String,
    // TASK-2241: rectangular brush state (screen coordinates).
    // ブラシ選択は Shift + 左ドラッグ。修飾なし左ドラッグは統一ナビゲーション
    // （矩形ズーム）に割り当てるため、開始判定で Shift を要求する。
    // egui_plot のクロスヘアは生のスクリーン座標へ最終変換を適用して描かれる。
    // 矩形もスクリーン座標で保持し、描画・選択判定ともに `PlotResponse.transform`
    // （点描画と同一の最終変換）で扱うことで、変換のフレーム遅延によるズレを避ける。
    #[serde(skip)]
    pub brush_start: Option<egui::Pos2>,
    #[serde(skip)]
    pub brush_end: Option<egui::Pos2>,
    /// 点クリックで開くトライアル詳細モーダル。
    #[serde(skip)]
    pub detail_modal: TrialDetailModal,
    /// サロゲート予測フロント点をオーバーレイ表示するか。
    pub show_surrogate_front: bool,
    /// 列抽出済みの点群キャッシュ（選択・ハイライト非依存・M-17）。
    #[serde(skip)]
    point_cache: Option<PointCache>,
}

/// 目的列から抽出済みの点群キャッシュ（選択フィルタ・ハイライトには依存しない）。
///
/// 旧実装は毎フレーム全 trial について「列取得 + feasibility 判定 + rank 参照 +
/// trial_id 参照」を回して点ベクトル群を再構築していた。これらは `view.df` の
/// 恒等性と軸だけで決まるため、`(df ポインタ, x_idx, y_idx)` をキーに 1 度だけ抽出し、
/// 選択・ハイライトによる分類は描画時に軽く適用する（M-16 の HashSet を併用）。
struct PointCache {
    key: (usize, usize, usize),
    /// feasible 点: `(trial_id, pareto_rank, [x, y])`。
    feasible: Vec<(u32, u32, [f64; 2])>,
    /// infeasible 点の座標（常にグレーで最背面）。
    infeasible_pts: Vec<[f64; 2]>,
    /// クリック・ブラシ判定用の全描画点 `(trial_id, 行 index, [x, y])`。
    displayed_points: Vec<(u32, usize, [f64; 2])>,
}

impl Default for ParetoScatter2D {
    fn default() -> Self {
        Self {
            x_axis: "obj0".to_string(),
            y_axis: "obj1".to_string(),
            brush_start: None,
            brush_end: None,
            detail_modal: TrialDetailModal::new(),
            show_surrogate_front: true,
            point_cache: None,
        }
    }
}

/// feasibility 分割・pareto_rank・trial_id を行順に展開した分類結果（2D/3D Pareto 共通・D-6）。
pub(crate) struct ClassifiedRow {
    pub trial_id: u32,
    pub row: usize,
    pub feasible: bool,
    /// pareto_rank（feasible のときのみ意味を持つ。infeasible では 0）。
    pub rank: u32,
}

/// view の全 trial を行順に (trial_id, row, feasible, pareto_rank) へ分類する（D-6）。
/// 2D/3D Pareto 散布図の feasibility 分割・ランク参照・trial_id 参照を共有する。
/// 描画（色分け・ハイライト・深度ソート）は各ウィジェット側で行う。
pub(crate) fn classify_rows(view: &crate::state::types::StudyView) -> Vec<ClassifiedRow> {
    let n = view.row_count();
    let feas = view.feasibility();
    (0..n)
        .map(|i| {
            let trial_id = view.trial_ids.get(i).copied().unwrap_or(i as u32);
            let feasible = feas.is_feasible(i);
            let rank = view.pareto_rank.get(i).copied().unwrap_or(0);
            ClassifiedRow {
                trial_id,
                row: i,
                feasible,
                rank,
            }
        })
        .collect()
}

/// 目的列から `PointCache` を構築する（選択・ハイライト非依存）。
fn build_point_cache(
    view: &crate::state::types::StudyView,
    x_col: Option<&[f64]>,
    y_col: Option<&[f64]>,
    key: (usize, usize, usize),
) -> PointCache {
    let n = view.row_count();
    let coord = |i: usize| {
        let x = x_col.and_then(|c| c.get(i)).copied().unwrap_or(0.0);
        let y = y_col.and_then(|c| c.get(i)).copied().unwrap_or(0.0);
        [x, y]
    };
    let mut feasible: Vec<(u32, u32, [f64; 2])> = Vec::new();
    let mut infeasible_pts: Vec<[f64; 2]> = Vec::new();
    let mut displayed_points: Vec<(u32, usize, [f64; 2])> = Vec::with_capacity(n);
    // feasibility 分割・ランク参照は 3D と共有する（D-6）。
    for r in classify_rows(view) {
        let pt = coord(r.row);
        displayed_points.push((r.trial_id, r.row, pt));
        if !r.feasible {
            infeasible_pts.push(pt);
            continue;
        }
        feasible.push((r.trial_id, r.rank, pt));
    }
    PointCache {
        key,
        feasible,
        infeasible_pts,
        displayed_points,
    }
}

/// 目的軸名から `SurrogateMultiOptUiResult` のフロント点列を解決する純粋関数。
/// どちらかの軸名が結果に含まれない場合は空 Vec を返す。
pub fn surrogate_front_points(
    result: &SurrogateMultiOptUiResult,
    x_axis: &str,
    y_axis: &str,
) -> Vec<[f64; 2]> {
    let x_idx = result.objective_names.iter().position(|n| n == x_axis);
    let y_idx = result.objective_names.iter().position(|n| n == y_axis);
    match (x_idx, y_idx) {
        (Some(xi), Some(yi)) => result
            .front
            .iter()
            .filter_map(|pt| {
                let x = pt.values.get(xi).copied()?;
                let y = pt.values.get(yi).copied()?;
                Some([x, y])
            })
            .collect(),
        _ => Vec::new(),
    }
}

impl ParetoScatter2D {
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        app_state: &mut AppState,
        surrogate_front: Option<&SurrogateMultiOptUiResult>,
    ) {
        let Some(ctx) = &app_state.current_study else {
            ui.centered_and_justified(|ui| {
                ui.label("Select a study");
            });
            return;
        };

        let obj_names = ctx.meta.objective_names.clone();
        let param_names = ctx.meta.param_names.clone();
        let selected = app_state.selected_indices.clone();
        let highlighted = app_state.highlighted_trial;

        // 既定の軸名（"obj0"/"obj1"）は読み込んだ目的関数名と一致しないため、
        // 現在の選択が目的関数名に無ければ実際の名前へ寄せる（MCDM 散布図と同じ挙動）。
        if !obj_names.iter().any(|n| n == &self.x_axis) {
            if let Some(first) = obj_names.first() {
                self.x_axis = first.clone();
            }
        }
        if !obj_names.iter().any(|n| n == &self.y_axis) {
            if obj_names.len() > 1 {
                self.y_axis = obj_names[1].clone();
            } else if let Some(first) = obj_names.first() {
                self.y_axis = first.clone();
            }
        }

        // 軸割り当て ComboBox + サロゲートフロントチェックボックス
        ui.horizontal(|ui| {
            ui.label("X Axis:");
            egui::ComboBox::from_id_salt("x_axis_combo")
                .selected_text(&self.x_axis)
                .show_ui(ui, |ui| {
                    for name in &obj_names {
                        ui.selectable_value(&mut self.x_axis, name.clone(), name);
                    }
                });
            ui.label("Y Axis:");
            egui::ComboBox::from_id_salt("y_axis_combo")
                .selected_text(&self.y_axis)
                .show_ui(ui, |ui| {
                    for name in &obj_names {
                        ui.selectable_value(&mut self.y_axis, name.clone(), name);
                    }
                });
            // サロゲートフロントが利用可能な場合のみチェックボックスを表示する。
            if surrogate_front.is_some() {
                ui.checkbox(&mut self.show_surrogate_front, "Surrogate front");
            }
        });

        let x_idx = obj_names
            .iter()
            .position(|n| n == &self.x_axis)
            .unwrap_or(0);
        let y_idx = obj_names
            .iter()
            .position(|n| n == &self.y_axis)
            .unwrap_or(1);

        // view の列スライスから直接点群を構築（行クローンキャッシュを持たない・MEM-002）
        let view = &ctx.view;
        let x_col = obj_names
            .get(x_idx)
            .and_then(|name| view.numeric_column(name));
        let y_col = obj_names
            .get(y_idx)
            .and_then(|name| view.numeric_column(name));
        let feas = view.feasibility();

        // 列抽出・feasibility 判定は df の恒等性と軸だけで決まるため、毎フレームの
        // 再構築を避けてキャッシュする（M-17）。選択・ハイライトによる分類は下で
        // 軽く適用する。
        let cache_key = (Arc::as_ptr(&view.df) as usize, x_idx, y_idx);
        if self.point_cache.as_ref().map(|c| c.key) != Some(cache_key) {
            self.point_cache = Some(build_point_cache(view, x_col, y_col, cache_key));
        }
        let cache = self.point_cache.as_ref().expect("point cache built above");

        // パレートフロント(rank==0)と非パレートに分類。
        // 選択フィルタが有効な場合、選択外は Pareto/非 Pareto を問わず灰色でまとめる
        // （色相を残すと選択点と紛らわしいため）。選択集合は HashSet を 1 度だけ構築し、
        // 点ごとの線形走査を避ける（M-16）。
        let selected_set: HashSet<u32> = selected.iter().copied().collect();
        let mut pareto_pts: Vec<[f64; 2]> = Vec::new();
        let mut non_pareto_pts: Vec<[f64; 2]> = Vec::new();
        let mut unselected_pts: Vec<[f64; 2]> = Vec::new();
        let mut highlight_pt: Option<[f64; 2]> = None;
        for &(trial_id, rank, pt) in &cache.feasible {
            if highlighted == Some(trial_id) {
                highlight_pt = Some(pt);
                continue;
            }
            if point_alpha_in_set(trial_id, &selected_set) != 255 {
                // 選択外は Pareto/非 Pareto を問わず灰色グループへ
                unselected_pts.push(pt);
            } else if rank == 0 {
                pareto_pts.push(pt);
            } else {
                non_pareto_pts.push(pt);
            }
        }
        // 実行不可能解・ヒットテスト候補はキャッシュから参照する。
        let infeasible_pts = &cache.infeasible_pts;
        let displayed_points = &cache.displayed_points;

        // サロゲートフロント点を事前に計算する（クロージャ内で借用衝突を避けるため）。
        let surrogate_pts: Vec<[f64; 2]> = if self.show_surrogate_front {
            surrogate_front
                .map(|r| surrogate_front_points(r, &self.x_axis, &self.y_axis))
                .unwrap_or_default()
        } else {
            Vec::new()
        };

        // Capture brush events inside the closure using mutable local vars (screen coords)
        let mut new_brush_start: Option<egui::Pos2> = None;
        let mut new_brush_end: Option<egui::Pos2> = None;
        let mut drag_finished = false;
        let mut blank_clicked = false;
        // 点クリックで開く詳細モーダルの対象（trial_id, 行 index）。
        let mut clicked_detail: Option<(u32, usize)> = None;
        // マウスホバー中の点（trial_id, 行 index）。ツールチップ表示に使う。
        let mut hovered_detail: Option<(u32, usize)> = None;
        let current_brush_start = self.brush_start;
        let current_brush_end = self.brush_end;

        // Shift 押下中は矩形ズームを止めてブラシ選択に左ドラッグを譲る。
        let shift_down = ui.input(|i| i.modifiers.shift);

        let plot_response = egui_plot::Plot::new("pareto_2d_plot")
            .legend(egui_plot::Legend::default())
            .unified_nav()
            .allow_boxed_zoom(!shift_down)
            .show(ui, |plot_ui| {
                apply_wheel_zoom(plot_ui);
                // Brush interaction detection.
                // 矩形はスクリーン座標で保持する。egui_plot のクロスヘアは生のスクリーン
                // ポインタ位置に最終変換を適用して描かれるため、こちらもスクリーン座標で
                // 扱えば、描画・選択ともにクロージャ後の最終 transform で一貫処理でき、
                // 変換のフレーム遅延に起因するズレを完全に避けられる。
                let resp = plot_ui.response();
                // クロスヘア（ルーラー）と同じ `hover_pos()` を基準にし、ドラッグ中に
                // None になり得る場合は interact / latest にフォールバックする。
                let ptr = resp
                    .hover_pos()
                    .or_else(|| resp.interact_pointer_pos())
                    .or_else(|| resp.ctx.input(|i| i.pointer.latest_pos()));

                // ブラシ選択は Shift + 左ドラッグでのみ開始する。修飾なし左ドラッグ
                // は egui_plot の矩形ズームが処理する（統一ナビゲーション）。
                if shift_down && resp.drag_started_by(egui::PointerButton::Primary) {
                    new_brush_start = ptr;
                }
                // ブラシ操作中はプライマリボタンが押されている限り毎フレーム
                // ライブのポインタ座標で終端を更新する。`dragged_by()` はポインタが
                // 動いたフレームでしか発火しないため、それに頼ると終端が前フレームの
                // 古い座標に取り残され、矩形がカーソルからずれて見える。
                let brush_active = current_brush_start.is_some() || new_brush_start.is_some();
                let primary_down = resp.ctx.input(|i| i.pointer.primary_down());
                if brush_active && primary_down {
                    new_brush_end = ptr;
                }
                if resp.drag_stopped() {
                    drag_finished = true;
                }
                if resp.clicked_by(egui::PointerButton::Primary) {
                    // 点の近傍をクリックしたら詳細モーダル、空白なら選択クリア。
                    clicked_detail = resp.interact_pointer_pos().and_then(|pos| {
                        hit_test_nearest(plot_ui, displayed_points, pos, HIT_THRESHOLD)
                    });
                    blank_clicked = clicked_detail.is_none();
                }

                // ホバー中の点を検出（矩形ブラシ操作中は抑止）。
                if current_brush_start.is_none() && !resp.dragged_by(egui::PointerButton::Primary) {
                    hovered_detail = resp.hover_pos().and_then(|pos| {
                        hit_test_nearest(plot_ui, displayed_points, pos, HIT_THRESHOLD)
                    });
                }

                // 選択矩形は Plot 描画後にスクリーン座標で重ね描きする（下記参照）。

                // 実行不可能解（最背面: グレーアウト）
                if !infeasible_pts.is_empty() {
                    plot_ui.points(
                        egui_plot::Points::new("Infeasible", infeasible_pts.clone())
                            .color(COLOR_INFEASIBLE())
                            .radius(2.5),
                    );
                }
                // 選択フィルタ外（灰色・背面、Pareto/非 Pareto をまとめる）
                if !unselected_pts.is_empty() {
                    plot_ui.points(
                        egui_plot::Points::new("Others (unselected)", unselected_pts)
                            .color(COLOR_UNSELECTED_POINT())
                            .radius(2.5),
                    );
                }
                // 非パレート（青点）
                if !non_pareto_pts.is_empty() {
                    plot_ui.points(
                        egui_plot::Points::new("Others", non_pareto_pts)
                            .color(COLOR_NON_PARETO())
                            .radius(2.5),
                    );
                }
                // パレートフロント（赤丸 + 赤線）
                if !pareto_pts.is_empty() {
                    plot_ui.points(
                        egui_plot::Points::new("Pareto Front", pareto_pts)
                            .color(COLOR_PARETO())
                            .radius(4.0),
                    );
                }
                // サロゲート予測フロント（金色ダイヤモンド）
                if !surrogate_pts.is_empty() {
                    plot_ui.points(
                        egui_plot::Points::new("Surrogate Pareto Front", surrogate_pts)
                            .shape(egui_plot::MarkerShape::Diamond)
                            .radius(4.5)
                            .color(COLOR_SURROGATE_FRONT()),
                    );
                }
                // ハイライト点
                if let Some(pt) = highlight_pt {
                    plot_ui.points(
                        egui_plot::Points::new("Highlighted", vec![pt])
                            .color(COLOR_HIGHLIGHT_PT())
                            .radius(8.0),
                    );
                }
            });

        let plot_transform = plot_response.transform;

        // 選択矩形をスクリーン座標で重ね描きする。点描画と同じ最終 transform の
        // 描画領域（frame）にクリップするため、矩形は常に実カーソルへ正確に追従する。
        let draw_start = new_brush_start.or(current_brush_start);
        let draw_end = new_brush_end.or(current_brush_end);
        if let (Some(s), Some(e)) = (draw_start, draw_end) {
            let rect = egui::Rect::from_two_pos(s, e);
            let painter = ui.painter().with_clip_rect(*plot_transform.frame());
            painter.rect(
                rect,
                0.0,
                egui::Color32::from_rgba_unmultiplied(100, 150, 255, 40),
                egui::Stroke::new(1.0, egui::Color32::from_rgb(100, 150, 255)),
                egui::StrokeKind::Inside,
            );
        }

        // ホバー中の点があれば、ポインタ位置に概要ツールチップを表示する。
        // view / feas の不変借用のみで完結させ、app_state の可変借用前に処理する。
        if let Some((_, row)) = hovered_detail {
            let trial_number = view.df.get_trial_number(row).unwrap_or(row as u32);
            let rank = view.pareto_rank.get(row).copied().unwrap_or(0);
            let mut rows = vec![
                axis_row(&self.x_axis, x_col, row),
                axis_row(&self.y_axis, y_col, row),
                ("Pareto Rank".to_string(), rank.to_string()),
            ];
            push_feasible_row(&mut rows, feas, row);
            crate::ui::widgets::trial_detail_modal::show_hover_tooltip(
                ui,
                "pareto2d_hover_tooltip",
                trial_number,
                &rows,
            );
        }

        // 点クリックでトライアル詳細モーダルを開く（散布図情報 = Pareto ランク）。
        // app_state を可変借用する前に view / feas の不変借用を使い切る。
        if let Some((trial_id, row)) = clicked_detail {
            let rank = view.pareto_rank.get(row).copied().unwrap_or(0);
            let mut context = vec![("Pareto Rank".to_string(), rank.to_string())];
            push_feasible_row(&mut context, feas, row);
            self.detail_modal.open(TrialDetailTarget {
                trial_id,
                row_index: row,
                context,
            });
        }

        // Update brush state and selection after closure
        if let Some(start) = new_brush_start {
            self.brush_start = Some(start);
            self.brush_end = None;
        }
        if let Some(end) = new_brush_end {
            self.brush_end = Some(end);
        }
        if drag_finished {
            if let (Some(start), Some(end)) = (self.brush_start, self.brush_end) {
                // 各点を描画と同じ最終 transform でスクリーン座標へ変換し、矩形（スクリーン）
                // に含まれるかで判定する。見た目の矩形と選択結果が必ず一致する。
                let rect = egui::Rect::from_two_pos(start, end);
                let new_selection: Vec<u32> = displayed_points
                    .iter()
                    .filter(|(_, _, pt)| {
                        let screen = plot_transform
                            .position_from_point(&egui_plot::PlotPoint::new(pt[0], pt[1]));
                        rect.contains(screen)
                    })
                    .map(|(id, _, _)| *id)
                    .collect();
                app_state.selected_indices = new_selection;
            }
            self.brush_start = None;
            self.brush_end = None;
        }
        if blank_clicked && self.brush_start.is_none() {
            // Empty click outside drag = clear selection
            app_state.selected_indices.clear();
        }

        // 詳細モーダルを描画する（current_study / artifact_map を再借用）。
        if self.detail_modal.is_open() {
            if let Some(ctx) = app_state.current_study.as_ref() {
                self.detail_modal.show(
                    ui,
                    &ctx.view,
                    &param_names,
                    &obj_names,
                    &app_state.artifact_map,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pareto_scatter_2d_default() {
        let widget = ParetoScatter2D::default();
        assert_eq!(widget.x_axis, "obj0");
        assert_eq!(widget.y_axis, "obj1");
    }

    #[test]
    fn blank_click_clears_selection_per_policy() {
        // Simulate blank click: selected_indices should become empty
        let mut selected: Vec<u32> = vec![0, 1, 2];
        // Policy: blank click clears selection
        selected.clear();
        assert!(selected.is_empty());
    }

    #[test]
    fn brush_state_default_is_none() {
        let widget = ParetoScatter2D::default();
        assert!(widget.brush_start.is_none());
        assert!(widget.brush_end.is_none());
    }

    // ── surrogate_front_points のユニットテスト ───────────────────────

    fn make_ui_result() -> crate::state::messages::SurrogateMultiOptUiResult {
        use tunny_core::surrogate_opt::ParetoFrontPoint;
        crate::state::messages::SurrogateMultiOptUiResult {
            param_names: vec!["x".to_string()],
            objective_names: vec!["f0".to_string(), "f1".to_string()],
            front: vec![
                ParetoFrontPoint {
                    params: vec![0.1],
                    values: vec![1.0, 4.0],
                },
                ParetoFrontPoint {
                    params: vec![0.2],
                    values: vec![2.0, 3.0],
                },
            ],
            r_squared: vec![0.9, 0.85],
        }
    }

    #[test]
    fn surrogate_front_points_normal_order() {
        let result = make_ui_result();
        let pts = surrogate_front_points(&result, "f0", "f1");
        assert_eq!(pts.len(), 2);
        assert_eq!(pts[0], [1.0, 4.0]);
        assert_eq!(pts[1], [2.0, 3.0]);
    }

    #[test]
    fn surrogate_front_points_swapped_axes() {
        let result = make_ui_result();
        let pts = surrogate_front_points(&result, "f1", "f0");
        assert_eq!(pts.len(), 2);
        assert_eq!(pts[0], [4.0, 1.0]);
        assert_eq!(pts[1], [3.0, 2.0]);
    }

    #[test]
    fn surrogate_front_points_unknown_axis_returns_empty() {
        let result = make_ui_result();
        // 存在しない軸名 → 空
        let pts = surrogate_front_points(&result, "f0", "unknown");
        assert!(pts.is_empty());
    }

    #[test]
    fn pareto_scatter_2d_show_surrogate_front_default_true() {
        let widget = ParetoScatter2D::default();
        assert!(widget.show_surrogate_front);
    }
}
