use crate::theme::chart_colors::{
    COLOR_CHART_TEXT, COLOR_INFEASIBLE, COLOR_PARALLEL_AXIS, COLOR_PARALLEL_LINE_DEFAULT,
    COLOR_PARALLEL_TICK,
};
use crate::theme::CENTRAL_BG;

/// 値の範囲に応じた精度で軸目盛り値をフォーマットする
pub fn fmt_tick_value(v: f64, mn: f64, mx: f64) -> String {
    let range = (mx - mn).abs();
    if range < 1e-9 {
        format!("{:.3}", v)
    } else if v.abs() >= 10_000.0 || (v.abs() < 0.001 && v.abs() > 0.0) {
        format!("{:.2e}", v)
    } else if range < 0.01 {
        format!("{:.4}", v)
    } else if range < 1.0 {
        format!("{:.3}", v)
    } else {
        format!("{:.2}", v)
    }
}

/// 値を [0, 1] に正規化する（min==max の場合は 0.5 を返す）
pub fn normalize_value(v: f64, v_min: f64, v_max: f64) -> f32 {
    if (v_max - v_min).abs() < f64::EPSILON {
        return 0.5;
    }
    ((v - v_min) / (v_max - v_min)).clamp(0.0, 1.0) as f32
}

/// 正規化値 [0,1] を画面の Y 座標に変換する（0 = bottom, 1 = top）
pub fn normalized_to_screen_y(normalized: f32, plot_top: f32, plot_bottom: f32) -> f32 {
    plot_bottom - normalized * (plot_bottom - plot_top)
}

/// 軸の表示名リストをパラメータ名と目的関数名から構築する
pub fn build_axis_order(param_names: &[String], objective_names: &[String]) -> Vec<String> {
    param_names
        .iter()
        .chain(objective_names.iter())
        .cloned()
        .collect()
}

/// 正規化ブラッシュ範囲をデータ値に逆変換する
pub fn denormalize_brush_range(
    y_min_normalized: f32,
    y_max_normalized: f32,
    axis_min: f64,
    axis_max: f64,
) -> (f64, f64) {
    let range = axis_max - axis_min;
    let raw_min = y_min_normalized as f64 * range + axis_min;
    let raw_max = y_max_normalized as f64 * range + axis_min;
    (raw_min, raw_max)
}

/// ドラッグ方向に関わらず (min, max) 順に整列する
pub fn ordered_brush_range(start: f32, end: f32) -> (f32, f32) {
    (start.min(end), start.max(end))
}

/// 平行座標図ウィジェット
pub struct ParallelCoordsChart {
    pub axis_order: Vec<String>,
    pub show_params: bool,
    pub show_objectives: bool,
    pub brush_ranges: std::collections::HashMap<String, Option<(f32, f32)>>,
    pub drag_start: Option<(String, f32)>,
    /// REQ-004: 軸ごとの表示/非表示フラグ（true = 表示）
    pub axis_visibility: std::collections::HashMap<String, bool>,
    col_ranges_cache: Option<Vec<(f64, f64)>>,
    cache_key: (usize, usize, usize), // (trial_count, n_params, n_objs)
    // TASK-2242: pending selection from completed brush drag
    pub pending_selection: Option<Vec<u32>>,
    /// 実行不可能解を表示するか（制約あり Study でのみ有効）
    pub show_infeasible: bool,
}

impl Default for ParallelCoordsChart {
    fn default() -> Self {
        Self {
            axis_order: Vec::new(),
            show_params: true,
            show_objectives: true,
            brush_ranges: std::collections::HashMap::new(),
            drag_start: None,
            axis_visibility: std::collections::HashMap::new(),
            col_ranges_cache: None,
            cache_key: (0, 0, 0),
            pending_selection: None,
            show_infeasible: true,
        }
    }
}

impl ParallelCoordsChart {
    pub fn new() -> Self {
        Self::default()
    }

    /// 全ブラッシュ範囲をリセットする
    pub fn clear_brushes(&mut self) {
        self.brush_ranges.clear();
        self.drag_start = None;
        self.pending_selection = None;
    }

    /// 平行座標プロットを描画する
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        view: &crate::state::app_state::StudyView,
        param_names: &[String],
        obj_names: &[String],
        chart_colors: &[egui::Color32],
    ) {
        let trial_count = view.row_count();
        if trial_count == 0 {
            ui.centered_and_justified(|ui| {
                ui.label(egui::RichText::new("No trial data.").weak());
            });
            return;
        }

        let all_names = build_axis_order(param_names, obj_names);
        let n_axes = all_names.len();
        if n_axes < 2 {
            return;
        }

        let n_params = param_names.len();
        // 各軸の列スライスを view から借用（コピーしない・MEM-003）
        let cols = view.numeric_columns(&all_names);

        let cache_key = (trial_count, n_params, obj_names.len());
        if self.col_ranges_cache.is_none() || self.cache_key != cache_key {
            let col_ranges: Vec<(f64, f64)> = cols
                .iter()
                .map(|data| match data {
                    Some(c) => {
                        let mn = c.iter().cloned().fold(f64::INFINITY, f64::min);
                        let mx = c.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                        (mn, mx)
                    }
                    None => (0.0, 1.0),
                })
                .collect();
            self.col_ranges_cache = Some(col_ranges);
            self.cache_key = cache_key;
        }
        let col_ranges = self.col_ranges_cache.as_ref().unwrap();

        let is_feasible_col = view.numeric_column("is_feasible");
        let has_constraints = is_feasible_col.is_some();

        // "Show Infeasible" トグル（制約あり Study のみ表示）
        if has_constraints {
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.show_infeasible, "Show Infeasible");
            });
        }

        let available = ui.available_rect_before_wrap();
        let axis_margin = 40.0_f32;
        let axis_top = available.min.y + 30.0;
        let axis_bottom = available.max.y - 10.0;
        let axis_x: Vec<f32> = (0..n_axes)
            .map(|i| {
                available.min.x
                    + axis_margin
                    + (available.width() - 2.0 * axis_margin) * i as f32 / (n_axes - 1) as f32
            })
            .collect();

        let painter = ui.painter().clone();

        painter.rect_filled(available, 0.0, CENTRAL_BG);

        let text_color = COLOR_CHART_TEXT;
        const N_TICKS: usize = 5;
        let tick_len = 4.0_f32;
        let tick_color = COLOR_PARALLEL_TICK;
        let tick_font = egui::FontId::proportional(9.0);

        let show_infeasible = self.show_infeasible;

        // 各試行を折れ線で描画（半透明）
        for t_idx in 0..trial_count {
            let feasible = is_feasible_col
                .and_then(|c| c.get(t_idx))
                .map(|&v| v > 0.5)
                .unwrap_or(true);

            if !feasible && !show_infeasible {
                continue;
            }

            let color = if feasible {
                let base_color = chart_colors
                    .get(t_idx)
                    .copied()
                    .unwrap_or(COLOR_PARALLEL_LINE_DEFAULT);
                egui::Color32::from_rgba_unmultiplied(
                    base_color.r(),
                    base_color.g(),
                    base_color.b(),
                    120,
                )
            } else {
                COLOR_INFEASIBLE
            };

            let mut points: Vec<egui::Pos2> = Vec::with_capacity(n_axes);
            let mut valid = true;
            for i in 0..n_axes {
                let val_opt = cols
                    .get(i)
                    .and_then(|c| c.as_ref())
                    .and_then(|c| c.get(t_idx))
                    .copied();
                let Some(val) = val_opt else {
                    valid = false;
                    break;
                };
                let (mn, mx) = col_ranges[i];
                let norm = normalize_value(val, mn, mx);
                let y = normalized_to_screen_y(norm, axis_top, axis_bottom);
                points.push(egui::pos2(axis_x[i], y));
            }
            if valid && points.len() >= 2 {
                for pair in points.windows(2) {
                    painter.line_segment([pair[0], pair[1]], egui::Stroke::new(0.8, color));
                }
            }
        }

        // 縦軸・ラベル・目盛りを最前面に描画
        for (i, name) in all_names.iter().enumerate() {
            let x = axis_x[i];
            painter.line_segment(
                [egui::pos2(x, axis_top), egui::pos2(x, axis_bottom)],
                egui::Stroke::new(1.5, COLOR_PARALLEL_AXIS),
            );
            painter.text(
                egui::pos2(x, available.min.y + 15.0),
                egui::Align2::CENTER_CENTER,
                name.as_str(),
                egui::FontId::proportional(10.0),
                text_color,
            );

            let (mn, mx) = col_ranges[i];
            for t in 0..N_TICKS {
                let frac = t as f32 / (N_TICKS - 1) as f32;
                let y = normalized_to_screen_y(frac, axis_top, axis_bottom);
                painter.line_segment(
                    [egui::pos2(x - tick_len, y), egui::pos2(x + tick_len, y)],
                    egui::Stroke::new(1.0, tick_color),
                );
                let val = mn + frac as f64 * (mx - mn);
                painter.text(
                    egui::pos2(x - tick_len - 2.0, y),
                    egui::Align2::RIGHT_CENTER,
                    fmt_tick_value(val, mn, mx),
                    tick_font.clone(),
                    tick_color,
                );
            }
        }

        // Draw brush range overlays
        for (i, name) in all_names.iter().enumerate() {
            if let Some(Some((y_lo, y_hi))) = self.brush_ranges.get(name.as_str()) {
                let x = axis_x[i];
                let screen_hi = normalized_to_screen_y(*y_hi, axis_top, axis_bottom);
                let screen_lo = normalized_to_screen_y(*y_lo, axis_top, axis_bottom);
                let brush_rect = egui::Rect::from_min_max(
                    egui::pos2(x - 6.0, screen_hi),
                    egui::pos2(x + 6.0, screen_lo),
                );
                painter.rect_filled(
                    brush_rect,
                    2.0,
                    egui::Color32::from_rgba_unmultiplied(100, 150, 255, 80),
                );
                painter.rect_stroke(
                    brush_rect,
                    2.0,
                    egui::Stroke::new(1.5, egui::Color32::from_rgb(100, 150, 255)),
                );
            }
        }

        let response = ui.allocate_rect(available, egui::Sense::click_and_drag());

        // Brush drag interaction
        if let Some(ptr) = response.interact_pointer_pos() {
            // Find closest axis
            let closest_axis_idx = axis_x
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| {
                    (ptr.x - **a)
                        .abs()
                        .partial_cmp(&(ptr.x - **b).abs())
                        .unwrap()
                })
                .map(|(i, _)| i);

            if let Some(axis_idx) = closest_axis_idx {
                let axis_name = all_names[axis_idx].clone();
                // Normalize pointer Y to [0, 1]
                let norm_y = ((axis_bottom - ptr.y) / (axis_bottom - axis_top)).clamp(0.0, 1.0);

                if response.drag_started() {
                    self.drag_start = Some((axis_name, norm_y));
                } else if response.dragged() {
                    if let Some((ref start_name, start_y)) = self.drag_start.clone() {
                        if *start_name == axis_name {
                            let (lo, hi) = ordered_brush_range(start_y, norm_y);
                            self.brush_ranges.insert(axis_name, Some((lo, hi)));
                        }
                    }
                } else if response.drag_stopped() {
                    self.drag_start = None;
                    // Compute selection from all active brush ranges
                    let new_sel = filter_trials_by_brushes(
                        &view.trial_ids,
                        &self.brush_ranges,
                        &cols,
                        col_ranges,
                        &all_names,
                    );
                    self.pending_selection = Some(new_sel);
                }
            }
        }

        // Clear brushes on right-click or double-click
        if response.secondary_clicked() || response.double_clicked() {
            self.brush_ranges.clear();
            self.pending_selection = Some(vec![]); // empty = no selection filter
        }
    }
}

/// 全ブラシ範囲に対して AND 条件でトライアルをフィルタリングし trial_id リストを返す（TASK-2242）
/// 列スライス（view 由来の借用）と trial_ids 並行配列から算出する（行クローン不要・MEM-003）。
pub fn filter_trials_by_brushes(
    trial_ids: &[u32],
    brush_ranges: &std::collections::HashMap<String, Option<(f32, f32)>>,
    cols: &[Option<&[f64]>],
    col_ranges: &[(f64, f64)],
    all_names: &[String],
) -> Vec<u32> {
    (0..trial_ids.len())
        .filter_map(|t_idx| {
            for (axis_idx, axis_name) in all_names.iter().enumerate() {
                let Some(Some((lo, hi))) = brush_ranges.get(axis_name.as_str()) else {
                    continue; // no active brush on this axis
                };
                let val = cols
                    .get(axis_idx)
                    .and_then(|c| c.as_ref())
                    .and_then(|c| c.get(t_idx))
                    .copied()?;
                let (mn, mx) = col_ranges.get(axis_idx).copied()?;
                let norm = normalize_value(val, mn, mx);
                if norm < *lo || norm > *hi {
                    return None; // outside brush range
                }
            }
            trial_ids.get(t_idx).copied()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_min_maps_to_zero() {
        let n = normalize_value(0.0, 0.0, 10.0);
        assert!((n - 0.0).abs() < 1e-6);
    }

    #[test]
    fn normalize_max_maps_to_one() {
        let n = normalize_value(10.0, 0.0, 10.0);
        assert!((n - 1.0).abs() < 1e-6);
    }

    #[test]
    fn normalize_equal_min_max_returns_half() {
        let n = normalize_value(5.0, 5.0, 5.0);
        assert!((n - 0.5).abs() < 1e-6);
    }

    #[test]
    fn normalize_clamps_out_of_range() {
        let below = normalize_value(-1.0, 0.0, 1.0);
        let above = normalize_value(2.0, 0.0, 1.0);
        assert!((below - 0.0).abs() < 1e-6);
        assert!((above - 1.0).abs() < 1e-6);
    }

    #[test]
    fn normalized_to_screen_y_zero_maps_to_bottom() {
        let y = normalized_to_screen_y(0.0, 100.0, 400.0);
        assert!((y - 400.0).abs() < 1e-3);
    }

    #[test]
    fn normalized_to_screen_y_one_maps_to_top() {
        let y = normalized_to_screen_y(1.0, 100.0, 400.0);
        assert!((y - 100.0).abs() < 1e-3);
    }

    #[test]
    fn build_axis_order_concatenates_params_then_objectives() {
        let params = vec!["x".to_string(), "y".to_string()];
        let objs = vec!["obj0".to_string()];
        let axes = build_axis_order(&params, &objs);
        assert_eq!(axes, vec!["x", "y", "obj0"]);
    }

    #[test]
    fn parallel_coords_chart_default() {
        let chart = ParallelCoordsChart::default();
        assert!(chart.axis_order.is_empty());
        assert!(chart.show_params);
        assert!(chart.show_objectives);
        assert!(chart.brush_ranges.is_empty());
        assert!(chart.drag_start.is_none());
    }

    // TASK-2022 tests

    #[test]
    fn denormalize_brush_range_min_zero_max_one() {
        let (raw_min, raw_max) = denormalize_brush_range(0.0, 1.0, 0.0, 10.0);
        assert!((raw_min - 0.0).abs() < 1e-6);
        assert!((raw_max - 10.0).abs() < 1e-6);
    }

    #[test]
    fn denormalize_brush_range_midpoint() {
        let (raw_min, raw_max) = denormalize_brush_range(0.25, 0.75, 0.0, 4.0);
        assert!((raw_min - 1.0).abs() < 1e-6);
        assert!((raw_max - 3.0).abs() < 1e-6);
    }

    #[test]
    fn ordered_brush_range_forward_drag() {
        let (min, max) = ordered_brush_range(0.2, 0.8);
        assert!((min - 0.2).abs() < 1e-6);
        assert!((max - 0.8).abs() < 1e-6);
    }

    #[test]
    fn ordered_brush_range_reverse_drag() {
        // Dragging upward: start > end
        let (min, max) = ordered_brush_range(0.8, 0.2);
        assert!((min - 0.2).abs() < 1e-6);
        assert!((max - 0.8).abs() < 1e-6);
    }

    #[test]
    fn clear_brushes_empties_ranges() {
        let mut chart = ParallelCoordsChart::default();
        chart.brush_ranges.insert("x".to_string(), Some((0.1, 0.9)));
        chart.drag_start = Some(("x".to_string(), 0.5));
        chart.clear_brushes();
        assert!(chart.brush_ranges.is_empty());
        assert!(chart.drag_start.is_none());
    }

    // TASK-2125 tests
    #[test]
    fn axis_visibility_filter() {
        use std::collections::HashMap;
        let mut visibility: HashMap<String, bool> = HashMap::new();
        visibility.insert("x1".to_string(), true);
        visibility.insert("x2".to_string(), false);
        visibility.insert("x3".to_string(), true);
        let axis_order = ["x1".to_string(), "x2".to_string(), "x3".to_string()];
        let visible: Vec<_> = axis_order
            .iter()
            .filter(|name| *visibility.get(*name).unwrap_or(&true))
            .collect();
        assert_eq!(visible.len(), 2);
        assert_eq!(visible[0], "x1");
        assert_eq!(visible[1], "x3");
    }

    #[test]
    fn axis_reorder_logic() {
        let mut axis_order = vec!["x1".to_string(), "x2".to_string(), "x3".to_string()];
        let dragged = "x1";
        let target_idx = 2;
        if let Some(from_idx) = axis_order.iter().position(|a| a == dragged) {
            let name = axis_order.remove(from_idx);
            let insert_at = target_idx.min(axis_order.len());
            axis_order.insert(insert_at, name);
        }
        assert_eq!(axis_order, vec!["x2", "x3", "x1"]);
    }

    #[test]
    fn axis_visibility_all_hidden() {
        use std::collections::HashMap;
        let mut visibility: HashMap<String, bool> = HashMap::new();
        visibility.insert("x1".to_string(), false);
        visibility.insert("x2".to_string(), false);
        let axis_order = ["x1".to_string(), "x2".to_string()];
        let visible: Vec<_> = axis_order
            .iter()
            .filter(|name| *visibility.get(*name).unwrap_or(&true))
            .collect();
        assert!(visible.is_empty());
    }

    #[test]
    fn axis_visibility_default_true_for_unknown() {
        use std::collections::HashMap;
        let visibility: HashMap<String, bool> = HashMap::new();
        let axis_order = ["unknown_axis".to_string()];
        let visible: Vec<_> = axis_order
            .iter()
            .filter(|name| *visibility.get(*name).unwrap_or(&true))
            .collect();
        assert_eq!(visible.len(), 1);
    }

    // ── constraint-aware visualization (TASK-2349) ──────────────────

    #[test]
    fn tc_cav_parallel_coords_show_infeasible_default_true() {
        let chart = ParallelCoordsChart::default();
        assert!(chart.show_infeasible);
    }

    // --- TASK-2242: PCP brush tests ---

    fn make_trial_with_params(id: u32, p: f64, obj: f64) -> crate::state::app_state::TrialRow {
        use crate::state::app_state::{TrialRow, TrialState};
        use std::collections::HashMap;
        let mut params = HashMap::new();
        params.insert("x".to_string(), p);
        TrialRow {
            trial_id: id,
            trial_number: id,
            params,
            objectives: vec![obj],
            pareto_rank: 0,
            cluster_id: None,
            state: TrialState::Complete,
            user_attrs: HashMap::new(),
        }
    }

    #[test]
    fn normalize_and_denormalize_brush_range_round_trip() {
        // normalize 3.0 in [0, 10] → 0.3, then denormalize back
        let norm = normalize_value(3.0, 0.0, 10.0);
        let (lo, hi) = denormalize_brush_range(norm, norm, 0.0, 10.0);
        assert!((lo - 3.0).abs() < 1e-4);
        assert!((hi - 3.0).abs() < 1e-4);
    }

    #[test]
    fn multi_axis_brush_applies_and_filter() {
        use std::collections::HashMap;
        let trial_ids = vec![0u32, 1, 2];
        // col_data: axis 0 = x, axis 1 = obj
        let col_data = [
            vec![2.0, 8.0, 2.0], // x values
            vec![5.0, 5.0, 9.0], // obj values
        ];
        let cols: Vec<Option<&[f64]>> =
            vec![Some(col_data[0].as_slice()), Some(col_data[1].as_slice())];
        let col_ranges = vec![(0.0_f64, 10.0_f64), (0.0_f64, 10.0_f64)];
        let all_names = vec!["x".to_string(), "obj".to_string()];

        let mut brush_ranges: HashMap<String, Option<(f32, f32)>> = HashMap::new();
        // x in [0.0, 0.5] = values 0..5 → trial 0 and 2 pass
        brush_ranges.insert("x".to_string(), Some((0.0, 0.5)));
        // obj in [0.0, 0.6] = values 0..6 → trial 0 passes; trial 2 (obj=9) fails
        brush_ranges.insert("obj".to_string(), Some((0.0, 0.6)));

        let sel =
            filter_trials_by_brushes(&trial_ids, &brush_ranges, &cols, &col_ranges, &all_names);
        assert_eq!(sel.len(), 1);
        assert_eq!(sel[0], 0);
    }

    #[test]
    fn clear_brushes_resets_selection_state() {
        let mut chart = ParallelCoordsChart::default();
        chart.brush_ranges.insert("x".to_string(), Some((0.2, 0.8)));
        chart.pending_selection = Some(vec![0, 1]);
        chart.clear_brushes();
        assert!(chart.brush_ranges.is_empty());
        assert!(chart.pending_selection.is_none());
    }
}
