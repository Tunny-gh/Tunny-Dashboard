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
    col_data_cache: Option<Vec<Vec<f64>>>,
    col_ranges_cache: Option<Vec<(f64, f64)>>,
    cache_key: (usize, usize, usize), // (trial_count, n_params, n_objs)
}

impl Default for ParallelCoordsChart {
    fn default() -> Self {
        Self {
            axis_order: Vec::new(),
            show_params: true,
            show_objectives: true,
            brush_ranges: std::collections::HashMap::new(),
            drag_start: None,
            col_data_cache: None,
            col_ranges_cache: None,
            cache_key: (0, 0, 0),
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
    }

    /// 平行座標プロットを描画する
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        trial_rows: &[crate::state::app_state::TrialRow],
        param_names: &[String],
        obj_names: &[String],
        chart_colors: &[egui::Color32],
    ) {
        if trial_rows.is_empty() {
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
        let cache_key = (trial_rows.len(), n_params, obj_names.len());
        if self.col_data_cache.is_none() || self.cache_key != cache_key {
            let col_data: Vec<Vec<f64>> = all_names
                .iter()
                .enumerate()
                .map(|(idx, name)| {
                    if idx < n_params {
                        trial_rows
                            .iter()
                            .filter_map(|r| r.params.get(name).copied())
                            .collect()
                    } else {
                        let obj_idx = idx - n_params;
                        trial_rows
                            .iter()
                            .filter_map(|r| r.objectives.get(obj_idx).copied())
                            .collect()
                    }
                })
                .collect();
            let col_ranges: Vec<(f64, f64)> = col_data
                .iter()
                .map(|data| {
                    let mn = data.iter().cloned().fold(f64::INFINITY, f64::min);
                    let mx = data.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                    (mn, mx)
                })
                .collect();
            self.col_data_cache = Some(col_data);
            self.col_ranges_cache = Some(col_ranges);
            self.cache_key = cache_key;
        }
        let col_data = self.col_data_cache.as_ref().unwrap();
        let col_ranges = self.col_ranges_cache.as_ref().unwrap();

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

        // 背景を白で塗りつぶす
        painter.rect_filled(available, 0.0, egui::Color32::WHITE);

        let text_color = egui::Color32::BLACK;
        const N_TICKS: usize = 5;
        let tick_len = 4.0_f32;
        let tick_color = egui::Color32::from_gray(60);
        let tick_font = egui::FontId::proportional(9.0);

        // 各試行を折れ線で描画（半透明）
        for t_idx in 0..trial_rows.len() {
            let base_color = chart_colors
                .get(t_idx)
                .copied()
                .unwrap_or(egui::Color32::from_rgb(100, 150, 220));
            let color = egui::Color32::from_rgba_unmultiplied(
                base_color.r(),
                base_color.g(),
                base_color.b(),
                120,
            );

            let mut points: Vec<egui::Pos2> = Vec::with_capacity(n_axes);
            let mut valid = true;
            for i in 0..n_axes {
                let val_opt = col_data[i].get(t_idx).copied();
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
                egui::Stroke::new(1.5, egui::Color32::from_gray(80)),
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

        ui.allocate_rect(available, egui::Sense::hover());
    }
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
}
