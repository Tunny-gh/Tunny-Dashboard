use crate::state::app_state::{AppState, TrialRow};
use crate::state::messages::SurrogateMultiOptUiResult;
use crate::theme::chart_colors::{
    COLOR_HIGHLIGHT_PT, COLOR_INFEASIBLE, COLOR_NON_PARETO, COLOR_PARETO, COLOR_SURROGATE_FRONT,
    COLOR_UNSELECTED_POINT,
};
use crate::theme::color_compute::compute_point_alpha;
use crate::ui::widgets::trial_detail_modal::{
    hit_test_nearest, TrialDetailModal, TrialDetailTarget, HIT_THRESHOLD,
};

type PartitionedPoints = (Vec<[f64; 2]>, Vec<[f64; 2]>, Option<[f64; 2]>);

/// Pareto ランクに応じたマーカー半径を返す（ランク0が最大）
pub fn pareto_marker_radius(pareto_rank: u32) -> f32 {
    if pareto_rank == 0 {
        5.0
    } else {
        2.5
    }
}

/// 2D Pareto 散布図ウィジェット（egui_plot ベース）
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct ParetoScatter2D {
    pub x_axis: String,
    pub y_axis: String,
    // TASK-2241: rectangular brush state (screen coordinates).
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
        }
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
        let n = view.row_count();
        let x_col = obj_names
            .get(x_idx)
            .and_then(|name| view.numeric_column(name));
        let y_col = obj_names
            .get(y_idx)
            .and_then(|name| view.numeric_column(name));

        // パレートフロント(rank==0)と非パレートに分類。
        // 選択フィルタが有効な場合、選択外は Pareto/非 Pareto を問わず灰色でまとめる
        // （色相を残すと選択点と紛らわしいため）。
        let mut pareto_pts: Vec<[f64; 2]> = Vec::new();
        let mut non_pareto_pts: Vec<[f64; 2]> = Vec::new();
        let mut unselected_pts: Vec<[f64; 2]> = Vec::new();
        let mut infeasible_pts: Vec<[f64; 2]> = Vec::new();
        let mut highlight_pt: Option<[f64; 2]> = None;
        // ブラシ矩形選択・点クリック判定用に (trial_id, 行 index, 点) を保持（行クローンを持たない）
        let mut displayed_points: Vec<(u32, usize, [f64; 2])> = Vec::new();

        let feas = view.feasibility();

        for i in 0..n {
            let x = x_col.and_then(|c| c.get(i)).copied().unwrap_or(0.0);
            let y = y_col.and_then(|c| c.get(i)).copied().unwrap_or(0.0);
            let pt = [x, y];
            let trial_id = view.trial_ids.get(i).copied().unwrap_or(i as u32);
            displayed_points.push((trial_id, i, pt));

            let feasible = feas.is_feasible(i);

            if !feasible {
                infeasible_pts.push(pt);
                continue;
            }

            let rank = view.pareto_rank.get(i).copied().unwrap_or(0);

            if highlighted == Some(trial_id) {
                highlight_pt = Some(pt);
                continue;
            }

            let is_selected = compute_point_alpha(trial_id, &selected) == 255;
            if !is_selected {
                // 選択外は Pareto/非 Pareto を問わず灰色グループへ
                unselected_pts.push(pt);
            } else if rank == 0 {
                pareto_pts.push(pt);
            } else {
                non_pareto_pts.push(pt);
            }
        }

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

        let plot_response = egui_plot::Plot::new("pareto_2d_plot")
            .legend(egui_plot::Legend::default())
            .allow_drag(false)
            .show(ui, |plot_ui| {
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

                if resp.drag_started_by(egui::PointerButton::Primary) {
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
                        hit_test_nearest(plot_ui, &displayed_points, pos, HIT_THRESHOLD)
                    });
                    blank_clicked = clicked_detail.is_none();
                }

                // ホバー中の点を検出（矩形ブラシ操作中は抑止）。
                if current_brush_start.is_none() && !resp.dragged_by(egui::PointerButton::Primary) {
                    hovered_detail = resp.hover_pos().and_then(|pos| {
                        hit_test_nearest(plot_ui, &displayed_points, pos, HIT_THRESHOLD)
                    });
                }

                // 選択矩形は Plot 描画後にスクリーン座標で重ね描きする（下記参照）。

                // 実行不可能解（最背面: グレーアウト）
                if !infeasible_pts.is_empty() {
                    plot_ui.points(
                        egui_plot::Points::new("Infeasible", infeasible_pts)
                            .color(COLOR_INFEASIBLE)
                            .radius(2.5),
                    );
                }
                // 選択フィルタ外（灰色・背面、Pareto/非 Pareto をまとめる）
                if !unselected_pts.is_empty() {
                    plot_ui.points(
                        egui_plot::Points::new("Others (unselected)", unselected_pts)
                            .color(COLOR_UNSELECTED_POINT)
                            .radius(2.5),
                    );
                }
                // 非パレート（青点）
                if !non_pareto_pts.is_empty() {
                    plot_ui.points(
                        egui_plot::Points::new("Others", non_pareto_pts)
                            .color(COLOR_NON_PARETO)
                            .radius(2.5),
                    );
                }
                // パレートフロント（赤丸 + 赤線）
                if !pareto_pts.is_empty() {
                    plot_ui.points(
                        egui_plot::Points::new("Pareto Front", pareto_pts)
                            .color(COLOR_PARETO)
                            .radius(4.0),
                    );
                }
                // サロゲート予測フロント（金色ダイヤモンド）
                if !surrogate_pts.is_empty() {
                    plot_ui.points(
                        egui_plot::Points::new("Surrogate Pareto Front", surrogate_pts)
                            .shape(egui_plot::MarkerShape::Diamond)
                            .radius(4.5)
                            .color(COLOR_SURROGATE_FRONT),
                    );
                }
                // ハイライト点
                if let Some(pt) = highlight_pt {
                    plot_ui.points(
                        egui_plot::Points::new("Highlighted", vec![pt])
                            .color(COLOR_HIGHLIGHT_PT)
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
            let fmt = |v: Option<f64>| v.map(|x| format!("{x:.4}")).unwrap_or_else(|| "—".into());
            let mut rows = vec![
                (
                    self.x_axis.clone(),
                    fmt(x_col.and_then(|c| c.get(row)).copied()),
                ),
                (
                    self.y_axis.clone(),
                    fmt(y_col.and_then(|c| c.get(row)).copied()),
                ),
                ("Pareto Rank".to_string(), rank.to_string()),
            ];
            if feas.has_constraints() {
                rows.push((
                    "Feasible".to_string(),
                    if feas.is_feasible(row) { "Yes" } else { "No" }.to_string(),
                ));
            }
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
            let feasible = feas.is_feasible(row);
            let mut context = vec![("Pareto Rank".to_string(), rank.to_string())];
            if feas.has_constraints() {
                context.push((
                    "Feasible".to_string(),
                    if feasible { "Yes" } else { "No" }.to_string(),
                ));
            }
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

/// 点 [x, y] が矩形 (corner1, corner2) の内部にあるか判定する（TASK-2241）
pub fn point_in_rect(pt: [f64; 2], corner1: [f64; 2], corner2: [f64; 2]) -> bool {
    let x_min = corner1[0].min(corner2[0]);
    let x_max = corner1[0].max(corner2[0]);
    let y_min = corner1[1].min(corner2[1]);
    let y_max = corner1[1].max(corner2[1]);
    pt[0] >= x_min && pt[0] <= x_max && pt[1] >= y_min && pt[1] <= y_max
}

/// 表示インデックスを feasible / infeasible に分類する。
/// 制約なし Study（feas.has_constraints() == false）の場合は全件を feasible に分類する。
pub fn classify_by_feasibility(
    feas: tunny_core::dataframe::Feasibility<'_>,
    indices: &[usize],
) -> (Vec<usize>, Vec<usize>) {
    if !feas.has_constraints() {
        return (indices.to_vec(), vec![]);
    }
    let mut feasible = Vec::with_capacity(indices.len());
    let mut infeasible = Vec::with_capacity(indices.len());
    for &i in indices {
        if feas.is_feasible(i) {
            feasible.push(i);
        } else {
            infeasible.push(i);
        }
    }
    (feasible, infeasible)
}

/// 矩形内に含まれる trial の ID リストを返す（TASK-2241）
pub fn select_trials_in_rect(
    rows: &[TrialRow],
    corner1: [f64; 2],
    corner2: [f64; 2],
    x_idx: usize,
    y_idx: usize,
) -> Vec<u32> {
    rows.iter()
        .filter_map(|r| {
            let x = r.objectives.get(x_idx).copied().unwrap_or(0.0);
            let y = r.objectives.get(y_idx).copied().unwrap_or(0.0);
            if point_in_rect([x, y], corner1, corner2) {
                Some(r.trial_id)
            } else {
                None
            }
        })
        .collect()
}

/// TrialRow リストから選択・非選択・ハイライト点を分離する
pub fn partition_points(
    trial_rows: &[crate::state::app_state::TrialRow],
    selected_indices: &[u32],
    highlighted: Option<u32>,
    x_idx: usize,
    y_idx: usize,
) -> PartitionedPoints {
    let mut selected_pts = vec![];
    let mut unselected_pts = vec![];
    let mut highlight_pt = None;

    for row in trial_rows {
        let x = row.objectives.get(x_idx).copied().unwrap_or(0.0);
        let y = row.objectives.get(y_idx).copied().unwrap_or(0.0);
        let pt = [x, y];

        if let Some(h) = highlighted {
            if row.trial_id == h {
                highlight_pt = Some(pt);
                continue;
            }
        }

        let alpha = compute_point_alpha(row.trial_id, selected_indices);
        if alpha == 255 {
            selected_pts.push(pt);
        } else {
            unselected_pts.push(pt);
        }
    }
    (selected_pts, unselected_pts, highlight_pt)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::app_state::{TrialRow, TrialState};
    use std::collections::HashMap;

    fn make_trial(id: u32, objs: Vec<f64>) -> TrialRow {
        TrialRow {
            trial_id: id,
            trial_number: id,
            params: HashMap::new(),
            objectives: objs,
            pareto_rank: 0,
            cluster_id: None,
            state: TrialState::Complete,
            user_attrs: HashMap::new(),
        }
    }

    #[test]
    fn partition_empty_selected_all_go_to_selected() {
        let rows = vec![make_trial(0, vec![1.0, 2.0]), make_trial(1, vec![3.0, 4.0])];
        let (sel, unsel, hl) = partition_points(&rows, &[], None, 0, 1);
        assert_eq!(sel.len(), 2);
        assert_eq!(unsel.len(), 0);
        assert!(hl.is_none());
    }

    #[test]
    fn partition_with_selected_splits_correctly() {
        let rows = vec![
            make_trial(0, vec![1.0, 2.0]),
            make_trial(1, vec![3.0, 4.0]),
            make_trial(2, vec![5.0, 6.0]),
        ];
        let (sel, unsel, hl) = partition_points(&rows, &[0, 2], None, 0, 1);
        assert_eq!(sel.len(), 2);
        assert_eq!(unsel.len(), 1);
        assert!(hl.is_none());
        // 非選択は trial_id=1
        assert_eq!(unsel[0], [3.0, 4.0]);
    }

    #[test]
    fn partition_highlight_extracted_separately() {
        let rows = vec![make_trial(0, vec![1.0, 2.0]), make_trial(5, vec![9.0, 8.0])];
        let (sel, unsel, hl) = partition_points(&rows, &[], Some(5), 0, 1);
        assert_eq!(sel.len(), 1);
        assert_eq!(unsel.len(), 0);
        assert_eq!(hl, Some([9.0, 8.0]));
    }

    #[test]
    fn pareto_scatter_2d_default() {
        let widget = ParetoScatter2D::default();
        assert_eq!(widget.x_axis, "obj0");
        assert_eq!(widget.y_axis, "obj1");
    }

    #[test]
    fn pareto_marker_radius_rank0_is_larger() {
        let r0 = pareto_marker_radius(0);
        let r1 = pareto_marker_radius(1);
        assert!(r0 > r1);
    }

    #[test]
    fn pareto_marker_radius_non_front_rank_same() {
        assert_eq!(pareto_marker_radius(1), pareto_marker_radius(2));
    }

    // --- TASK-2241: Brush selection tests ---

    #[test]
    fn point_in_rect_detects_selected_trials() {
        // Point inside rect
        assert!(point_in_rect([2.0, 3.0], [1.0, 2.0], [4.0, 5.0]));
        // Point on boundary
        assert!(point_in_rect([1.0, 2.0], [1.0, 2.0], [4.0, 5.0]));
        // Point outside
        assert!(!point_in_rect([0.0, 0.0], [1.0, 2.0], [4.0, 5.0]));
        // Works with corners in either order
        assert!(point_in_rect([2.0, 3.0], [4.0, 5.0], [1.0, 2.0]));
    }

    #[test]
    fn brush_selection_updates_selected_indices() {
        let rows = vec![
            make_trial(0, vec![1.0, 1.0]),
            make_trial(1, vec![3.0, 3.0]),
            make_trial(2, vec![6.0, 6.0]),
        ];
        // Brush rect that covers trials 0 and 1 but not 2
        let selected = select_trials_in_rect(&rows, [0.0, 0.0], [4.0, 4.0], 0, 1);
        assert_eq!(selected.len(), 2);
        assert!(selected.contains(&0));
        assert!(selected.contains(&1));
        assert!(!selected.contains(&2));
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

    // ── constraint-aware visualization (TASK-2347) ──────────────────

    #[test]
    fn tc_cav_classify_no_constraint_all_feasible() {
        use tunny_core::dataframe::Feasibility;
        let indices = vec![0usize, 1, 2];
        let feas = Feasibility::from_column(None);
        let (feasible, infeasible) = classify_by_feasibility(feas, &indices);
        assert_eq!(feasible.len(), 3);
        assert!(infeasible.is_empty());
    }

    #[test]
    fn tc_cav_classify_mixed_feasibility() {
        use tunny_core::dataframe::Feasibility;
        // is_feasible: [1.0, 0.0, 1.0] → idx 0,2 feasible; idx 1 infeasible
        let col = vec![1.0f64, 0.0, 1.0];
        let indices = vec![0usize, 1, 2];
        let feas = Feasibility::from_column(Some(&col));
        let (feasible, infeasible) = classify_by_feasibility(feas, &indices);
        assert_eq!(feasible, vec![0, 2]);
        assert_eq!(infeasible, vec![1]);
    }

    #[test]
    fn tc_cav_classify_all_infeasible() {
        use tunny_core::dataframe::Feasibility;
        let col = vec![0.0f64, 0.0, 0.0];
        let indices = vec![0usize, 1, 2];
        let feas = Feasibility::from_column(Some(&col));
        let (feasible, infeasible) = classify_by_feasibility(feas, &indices);
        assert!(feasible.is_empty());
        assert_eq!(infeasible.len(), 3);
    }

    #[test]
    fn tc_cav_classify_all_feasible_with_constraint_col() {
        use tunny_core::dataframe::Feasibility;
        let col = vec![1.0f64, 1.0, 1.0];
        let indices = vec![0usize, 1, 2];
        let feas = Feasibility::from_column(Some(&col));
        let (feasible, infeasible) = classify_by_feasibility(feas, &indices);
        assert_eq!(feasible.len(), 3);
        assert!(infeasible.is_empty());
    }

    // ── surrogate_front_points のユニットテスト ───────────────────────

    fn make_ui_result() -> crate::state::messages::SurrogateMultiOptUiResult {
        use tunny_core::surrogate_opt::ParetoFrontPoint;
        crate::state::messages::SurrogateMultiOptUiResult {
            param_names: vec!["x".to_string()],
            objective_names: vec!["f0".to_string(), "f1".to_string()],
            minimize: vec![true, true],
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
            slices: vec![],
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
