use crate::theme::chart_colors::{
    COLOR_CHART_TEXT, COLOR_GRID_STROKE, COLOR_INFEASIBLE, COLOR_SCATTER_DOT,
};
use crate::theme::color_compute::correlation_color;

/// Scatter Matrix の1セルタイプ
#[derive(Debug, Clone, PartialEq)]
pub enum CellType {
    Scatter,     // 下三角セル（散布図）
    Correlation, // 上三角セル（相関係数）
    Histogram,   // 対角セル
}

/// Scatter Matrix の表示モード
#[derive(Debug, Clone, PartialEq)]
pub enum MatrixMode {
    ParamsVsParams,
    ParamsVsObjectives,
}

/// Scatter Matrix の軸ソート
#[derive(Debug, Clone, PartialEq)]
pub enum AxisSort {
    Alphabetical,
    Correlation,
}

/// Scatter Matrix の全体状態
pub struct ScatterMatrix {
    pub mode: MatrixMode,
    pub sort: AxisSort,
    pub selected_cell: Option<(usize, usize)>,
    /// 実行不可能解を表示するか（制約あり Study でのみ有効）
    pub show_infeasible: bool,
}

impl Default for ScatterMatrix {
    fn default() -> Self {
        Self {
            mode: MatrixMode::ParamsVsParams,
            sort: AxisSort::Alphabetical,
            selected_cell: None,
            show_infeasible: true,
        }
    }
}

impl ScatterMatrix {
    pub fn new() -> Self {
        Self::default()
    }

    /// 散布図行列を描画する
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

        let all_names: Vec<String> = param_names
            .iter()
            .chain(obj_names.iter())
            .cloned()
            .collect();
        let n = all_names.len();
        if n == 0 {
            return;
        }

        // 各軸の列スライスを view から借用（コピーしない・MEM-003）
        let cols: Vec<&[f64]> = all_names
            .iter()
            .map(|name| view.numeric_column(name).unwrap_or(&[]))
            .collect();

        let is_feasible_col = view.numeric_column("is_feasible");
        let has_constraints = is_feasible_col.is_some();

        // "Show Infeasible" トグル（制約あり Study のみ表示）
        if has_constraints {
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.show_infeasible, "Show Infeasible");
            });
        }

        let (feasible_indices, infeasible_indices) =
            split_feasibility_indices(trial_count, is_feasible_col);
        let show_infeasible = self.show_infeasible;

        // 行・列ラベルを事前レイアウトしてサイズを測る
        let outer = ui.available_rect_before_wrap();
        let painter = ui.painter().clone();
        let label_color = ui.visuals().text_color();
        let label_font = egui::FontId::proportional(10.0);
        let label_galleys: Vec<std::sync::Arc<egui::Galley>> = all_names
            .iter()
            .map(|name| painter.layout_no_wrap(name.clone(), label_font.clone(), label_color))
            .collect();
        let max_label_w = label_galleys
            .iter()
            .map(|g| g.size().x)
            .fold(0.0_f32, f32::max);
        let label_h = label_galleys.first().map(|g| g.size().y).unwrap_or(12.0);

        let label_angle = std::f32::consts::FRAC_PI_4; // 45°

        // 1セルの高さを見積もり、ラベルが行に収まらなければ行ラベルを 45° 回転
        let cell_h_est = outer.height() / n as f32;
        let rotate_rows = label_h > cell_h_est - 2.0 || max_label_w > outer.width() * 0.25;
        // 行ラベル（左端）の確保幅。回転時は対角方向の幅（最大110px）
        let row_label_w = if rotate_rows {
            (max_label_w * label_angle.cos() + label_h * label_angle.sin()).min(110.0) + 6.0
        } else {
            (max_label_w + 8.0).min(outer.width() * 0.25)
        };
        // グリッド幅から1セル幅を見積もり、ラベルが収まらなければ列ラベルを 45° 回転
        let grid_w_est = outer.width() - row_label_w;
        let cell_w_est = grid_w_est / n as f32;
        let rotate_cols = max_label_w > cell_w_est - 4.0;
        let col_label_h = if rotate_cols {
            (max_label_w * label_angle.sin() + label_h * label_angle.cos()).min(110.0) + 6.0
        } else {
            label_h + 6.0
        };

        let available = egui::Rect::from_min_max(
            egui::pos2(outer.min.x + row_label_w, outer.min.y + col_label_h),
            outer.max,
        );
        let cell_w = available.width() / n as f32;
        let cell_h = available.height() / n as f32;

        // 列ヘッダ（上端）と行ヘッダ（左端）に軸名を描画する
        for (idx, galley) in label_galleys.iter().enumerate() {
            let col_center_x = available.min.x + (idx as f32 + 0.5) * cell_w;
            let size = galley.size();
            if rotate_cols {
                // -45°（反時計回り）で回転させた "/" 形ラベルの最下端を
                // 各列中心・グリッド上端のすぐ上に合わせる（PCP と同じ手法）
                let applied = -label_angle;
                let (sa, ca) = (applied.sin(), applied.cos());
                let corners = [(0.0, 0.0), (size.x, 0.0), (0.0, size.y), (size.x, size.y)];
                let mut lowest = (0.0_f32, f32::MIN);
                for (px, py) in corners {
                    let rx = px * ca - py * sa;
                    let ry = px * sa + py * ca;
                    if ry > lowest.1 {
                        lowest = (rx, ry);
                    }
                }
                let anchor = egui::pos2(col_center_x, available.min.y - 2.0);
                let pos = anchor - egui::vec2(lowest.0, lowest.1);
                painter.add(
                    egui::epaint::TextShape::new(pos, galley.clone(), label_color)
                        .with_angle(applied),
                );
            } else {
                painter.galley(
                    egui::pos2(col_center_x - size.x * 0.5, available.min.y - label_h - 2.0),
                    galley.clone(),
                    label_color,
                );
            }

            let row_center_y = available.min.y + (idx as f32 + 0.5) * cell_h;
            if rotate_rows {
                // -45° で回転させたラベルの右端（最大 rx の隅）を、
                // 各行中心・グリッド左端のすぐ左に合わせる。
                let applied = -label_angle;
                let (sa, ca) = (applied.sin(), applied.cos());
                let corners = [(0.0, 0.0), (size.x, 0.0), (0.0, size.y), (size.x, size.y)];
                let mut right = (f32::MIN, 0.0); // (rx, ry) で rx 最大の隅
                let (mut min_ry, mut max_ry) = (f32::MAX, f32::MIN);
                for (px, py) in corners {
                    let rx = px * ca - py * sa;
                    let ry = px * sa + py * ca;
                    if rx > right.0 {
                        right = (rx, ry);
                    }
                    min_ry = min_ry.min(ry);
                    max_ry = max_ry.max(ry);
                }
                // 右端を (available.min.x - gap) に、回転後の縦中心を row_center_y に合わせる
                let anchor = egui::pos2(available.min.x - 4.0, row_center_y);
                let center_ry = (min_ry + max_ry) * 0.5;
                let pos = anchor - egui::vec2(right.0, center_ry);
                painter.add(
                    egui::epaint::TextShape::new(pos, galley.clone(), label_color)
                        .with_angle(applied),
                );
            } else {
                painter.galley(
                    egui::pos2(available.min.x - size.x - 4.0, row_center_y - size.y * 0.5),
                    galley.clone(),
                    label_color,
                );
            }
        }
        let dot_color = COLOR_SCATTER_DOT;
        let point_colors: Vec<egui::Color32> = if chart_colors.is_empty() {
            vec![dot_color; trial_count]
        } else {
            chart_colors.to_vec()
        };
        let infeasible_colors: Vec<egui::Color32> = vec![COLOR_INFEASIBLE; trial_count];

        for row in 0..n {
            for col in 0..n {
                let min = available.min + egui::vec2(col as f32 * cell_w, row as f32 * cell_h);
                let cell_rect = egui::Rect::from_min_size(min, egui::vec2(cell_w, cell_h));

                if row == col {
                    draw_histogram_cell(&painter, cell_rect, cols[row], 10);
                } else if col > row {
                    // 上三角: 相関係数
                    draw_correlation_cell(&painter, cell_rect, cols[row], cols[col]);
                } else {
                    // 下三角: 散布図
                    if has_constraints {
                        // infeasible を背面に描画（show_infeasible=true のみ）
                        if show_infeasible && !infeasible_indices.is_empty() {
                            draw_scatter_cell(
                                &painter,
                                cell_rect,
                                cols[col],
                                cols[row],
                                &infeasible_colors,
                                Some(&infeasible_indices),
                            );
                        }
                        // feasible を前面に描画
                        draw_scatter_cell(
                            &painter,
                            cell_rect,
                            cols[col],
                            cols[row],
                            &point_colors,
                            Some(&feasible_indices),
                        );
                    } else {
                        draw_scatter_cell(
                            &painter,
                            cell_rect,
                            cols[col],
                            cols[row],
                            &point_colors,
                            None,
                        );
                    }
                }

                // 各セルに枠線を描画してセル境界を明示する
                // （高密度の散布図でも図の範囲が分かるように）
                painter.rect_stroke(cell_rect, 0.0, egui::Stroke::new(0.5, COLOR_GRID_STROKE));
            }
        }

        ui.allocate_rect(outer, egui::Sense::hover());
    }
}

/// is_feasible 列から infeasible / feasible インデックスリストを構築する。
/// `is_feasible_col` が None の場合は全件を feasible 扱いとする。
pub fn split_feasibility_indices(
    n: usize,
    is_feasible_col: Option<&[f64]>,
) -> (Vec<u32>, Vec<u32>) {
    match is_feasible_col {
        None => {
            let all: Vec<u32> = (0..n as u32).collect();
            (all, vec![])
        }
        Some(col) => {
            let mut feasible = Vec::with_capacity(n);
            let mut infeasible = Vec::with_capacity(n);
            for i in 0..n {
                if col.get(i).map(|&v| v > 0.5).unwrap_or(true) {
                    feasible.push(i as u32);
                } else {
                    infeasible.push(i as u32);
                }
            }
            (feasible, infeasible)
        }
    }
}

/// モードに基づいてセルの行数・列数を計算する
pub fn grid_dimensions(mode: &MatrixMode, n_params: usize, n_objectives: usize) -> (usize, usize) {
    match mode {
        MatrixMode::ParamsVsParams => (n_params, n_params),
        MatrixMode::ParamsVsObjectives => (n_params, n_objectives),
    }
}

/// アルファベット順に軸名をソートする
pub fn sort_axes_alphabetical(axes: &mut [String]) {
    axes.sort();
}

/// 相関係数の絶対値が大きい順に軸をソートする（最初の軸との相関で順位付け）
/// axes: 軸名リスト, corr_matrix: axes[i] vs axes[j] の相関係数行列
pub fn sort_axes_by_correlation(axes: &mut [String], corr_matrix: &[Vec<f64>]) {
    if axes.is_empty() || corr_matrix.is_empty() {
        return;
    }
    // 最初の軸に対する絶対相関係数の合計でソート（降順）
    let n = axes.len().min(corr_matrix.len());
    let mut indexed: Vec<(usize, f64)> = (0..n)
        .map(|i| {
            let sum: f64 = corr_matrix[i][..n.min(corr_matrix[i].len())]
                .iter()
                .map(|&c| c.abs())
                .sum();
            (i, sum)
        })
        .collect();
    indexed.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    let old_axes: Vec<String> = axes.to_vec();
    for (new_pos, (old_idx, _)) in indexed.iter().enumerate() {
        axes[new_pos] = old_axes[*old_idx].clone();
    }
}

/// データ座標を画面座標に変換する
pub fn data_to_screen(
    x: f64,
    y: f64,
    x_range: (f64, f64),
    y_range: (f64, f64),
    cell_rect: egui::Rect,
) -> egui::Pos2 {
    let (x_min, x_max) = x_range;
    let (y_min, y_max) = y_range;
    let tx = if (x_max - x_min).abs() < f64::EPSILON {
        0.5
    } else {
        ((x - x_min) / (x_max - x_min)).clamp(0.0, 1.0)
    } as f32;
    let ty = if (y_max - y_min).abs() < f64::EPSILON {
        0.5
    } else {
        1.0 - ((y - y_min) / (y_max - y_min)).clamp(0.0, 1.0)
    } as f32;
    egui::pos2(
        cell_rect.left() + tx * cell_rect.width(),
        cell_rect.top() + ty * cell_rect.height(),
    )
}

/// ヒストグラムのビンカウントを計算する
pub fn compute_histogram(data: &[f64], n_bins: usize) -> Vec<usize> {
    if data.is_empty() || n_bins == 0 {
        return vec![0; n_bins];
    }
    let v_min = data.iter().cloned().fold(f64::INFINITY, f64::min);
    let v_max = data.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    if (v_max - v_min).abs() < f64::EPSILON {
        let mut bins = vec![0usize; n_bins];
        bins[n_bins / 2] = data.len();
        return bins;
    }
    let mut bins = vec![0usize; n_bins];
    for &v in data {
        let idx = ((v - v_min) / (v_max - v_min) * n_bins as f64) as usize;
        let idx = idx.min(n_bins - 1);
        bins[idx] += 1;
    }
    bins
}

/// Pearson 相関係数を計算する
pub fn compute_correlation(x: &[f64], y: &[f64]) -> f64 {
    let n = x.len().min(y.len());
    if n < 2 {
        return 0.0;
    }
    let mean_x = x[..n].iter().sum::<f64>() / n as f64;
    let mean_y = y[..n].iter().sum::<f64>() / n as f64;
    let cov: f64 = x[..n]
        .iter()
        .zip(y[..n].iter())
        .map(|(&xi, &yi)| (xi - mean_x) * (yi - mean_y))
        .sum::<f64>()
        / n as f64;
    let std_x: f64 =
        (x[..n].iter().map(|&xi| (xi - mean_x).powi(2)).sum::<f64>() / n as f64).sqrt();
    let std_y: f64 =
        (y[..n].iter().map(|&yi| (yi - mean_y).powi(2)).sum::<f64>() / n as f64).sqrt();
    if std_x < f64::EPSILON || std_y < f64::EPSILON {
        return 0.0;
    }
    (cov / (std_x * std_y)).clamp(-1.0, 1.0)
}

/// 散布図セルを painter で描画する
pub fn draw_scatter_cell(
    painter: &egui::Painter,
    cell_rect: egui::Rect,
    x_data: &[f64],
    y_data: &[f64],
    colors: &[egui::Color32],
    downsample_indices: Option<&[u32]>,
) {
    let x_min = x_data.iter().cloned().fold(f64::INFINITY, f64::min);
    let x_max = x_data.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let y_min = y_data.iter().cloned().fold(f64::INFINITY, f64::min);
    let y_max = y_data.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

    let indices: Box<dyn Iterator<Item = usize>> = if let Some(ds) = downsample_indices {
        Box::new(ds.iter().map(|&i| i as usize))
    } else {
        Box::new(0..x_data.len())
    };

    for i in indices {
        if i >= x_data.len() || i >= y_data.len() {
            continue;
        }
        let pos = data_to_screen(
            x_data[i],
            y_data[i],
            (x_min, x_max),
            (y_min, y_max),
            cell_rect,
        );
        let color = colors.get(i).copied().unwrap_or(COLOR_SCATTER_DOT);
        painter.circle_filled(pos, 1.3, color);
    }
}

/// ヒストグラムセルを painter で描画する
pub fn draw_histogram_cell(
    painter: &egui::Painter,
    cell_rect: egui::Rect,
    data: &[f64],
    n_bins: usize,
) {
    let bins = compute_histogram(data, n_bins);
    let max_count = *bins.iter().max().unwrap_or(&1).max(&1);
    let bar_width = cell_rect.width() / n_bins as f32;

    for (i, &count) in bins.iter().enumerate() {
        let bar_height = (count as f32 / max_count as f32) * cell_rect.height();
        let bar_rect = egui::Rect::from_min_size(
            egui::pos2(
                cell_rect.left() + i as f32 * bar_width,
                cell_rect.bottom() - bar_height,
            ),
            egui::vec2(bar_width - 1.0, bar_height),
        );
        painter.rect_filled(bar_rect, 0.0, COLOR_SCATTER_DOT);
    }
}

/// 相関係数セルを painter で描画する
pub fn draw_correlation_cell(
    painter: &egui::Painter,
    cell_rect: egui::Rect,
    x_data: &[f64],
    y_data: &[f64],
) {
    let corr = compute_correlation(x_data, y_data);
    let bg_color = correlation_color(corr);
    painter.rect_filled(cell_rect, 0.0, bg_color);
    painter.text(
        cell_rect.center(),
        egui::Align2::CENTER_CENTER,
        format!("{:.2}", corr),
        egui::FontId::proportional(12.0),
        COLOR_CHART_TEXT,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── constraint-aware visualization (TASK-2350) ──────────────────

    #[test]
    fn tc_cav_scatter_matrix_show_infeasible_default_true() {
        let sm = ScatterMatrix::default();
        assert!(sm.show_infeasible);
    }

    #[test]
    fn tc_cav_split_feasibility_no_constraints_all_feasible() {
        let (f, inf) = split_feasibility_indices(3, None);
        assert_eq!(f, vec![0, 1, 2]);
        assert!(inf.is_empty());
    }

    #[test]
    fn tc_cav_split_feasibility_mixed() {
        let col = vec![1.0_f64, 0.0, 1.0];
        let (f, inf) = split_feasibility_indices(3, Some(&col));
        assert_eq!(f, vec![0, 2]);
        assert_eq!(inf, vec![1]);
    }

    #[test]
    fn tc_cav_split_feasibility_all_infeasible() {
        let col = vec![0.0_f64, 0.0];
        let (f, inf) = split_feasibility_indices(2, Some(&col));
        assert!(f.is_empty());
        assert_eq!(inf, vec![0, 1]);
    }

    // TASK-2019 tests

    #[test]
    fn grid_dimensions_params_vs_params() {
        let (rows, cols) = grid_dimensions(&MatrixMode::ParamsVsParams, 4, 2);
        assert_eq!(rows, 4);
        assert_eq!(cols, 4);
    }

    #[test]
    fn grid_dimensions_params_vs_objectives() {
        let (rows, cols) = grid_dimensions(&MatrixMode::ParamsVsObjectives, 4, 2);
        assert_eq!(rows, 4);
        assert_eq!(cols, 2);
    }

    #[test]
    fn sort_axes_alphabetical_sorts_ascending() {
        let mut axes = vec!["z".to_string(), "a".to_string(), "m".to_string()];
        sort_axes_alphabetical(&mut axes);
        assert_eq!(axes, vec!["a", "m", "z"]);
    }

    #[test]
    fn sort_axes_by_correlation_highest_sum_first() {
        let mut axes = vec!["x".to_string(), "y".to_string(), "z".to_string()];
        // z has highest absolute correlation sum
        let corr_matrix = vec![
            vec![1.0, 0.1, 0.2], // x
            vec![0.1, 1.0, 0.3], // y
            vec![0.2, 0.3, 1.0], // z → sum = 1.5
        ];
        sort_axes_by_correlation(&mut axes, &corr_matrix);
        // z (sum=1.5) > y (sum=1.4) > x (sum=1.3)
        assert_eq!(axes[0], "z");
    }

    #[test]
    fn scatter_matrix_default_mode() {
        let sm = ScatterMatrix::default();
        assert_eq!(sm.mode, MatrixMode::ParamsVsParams);
        assert_eq!(sm.sort, AxisSort::Alphabetical);
        assert!(sm.selected_cell.is_none());
    }

    #[test]
    fn compute_histogram_bins_count() {
        let data = vec![0.0, 0.5, 1.0, 1.5, 2.0];
        let bins = compute_histogram(&data, 5);
        assert_eq!(bins.len(), 5);
        let total: usize = bins.iter().sum();
        assert_eq!(total, data.len());
    }

    #[test]
    fn compute_histogram_all_in_same_bin() {
        let data = vec![5.0; 10];
        let bins = compute_histogram(&data, 4);
        let total: usize = bins.iter().sum();
        assert_eq!(total, 10);
    }

    #[test]
    fn compute_histogram_empty_data() {
        let bins = compute_histogram(&[], 5);
        assert_eq!(bins.len(), 5);
        assert!(bins.iter().all(|&b| b == 0));
    }

    #[test]
    fn compute_correlation_perfect_positive() {
        let x: Vec<f64> = (0..10).map(|i| i as f64).collect();
        let y = x.clone();
        let corr = compute_correlation(&x, &y);
        assert!((corr - 1.0).abs() < 1e-9);
    }

    #[test]
    fn compute_correlation_perfect_negative() {
        let x: Vec<f64> = (0..10).map(|i| i as f64).collect();
        let y: Vec<f64> = x.iter().map(|&v| -v).collect();
        let corr = compute_correlation(&x, &y);
        assert!((corr + 1.0).abs() < 1e-9);
    }

    #[test]
    fn compute_correlation_range_bounded() {
        let x = vec![1.0, 3.0, 5.0, 7.0, 9.0];
        let y = vec![2.0, 1.0, 4.0, 3.0, 5.0];
        let corr = compute_correlation(&x, &y);
        assert!((-1.0..=1.0).contains(&corr));
    }

    #[test]
    fn data_to_screen_min_maps_to_left_bottom() {
        let rect = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(100.0, 100.0));
        let pos = data_to_screen(0.0, 0.0, (0.0, 1.0), (0.0, 1.0), rect);
        assert!((pos.x - 0.0).abs() < 1e-3);
        assert!((pos.y - 100.0).abs() < 1e-3); // y is inverted
    }

    #[test]
    fn data_to_screen_max_maps_to_right_top() {
        let rect = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(100.0, 100.0));
        let pos = data_to_screen(1.0, 1.0, (0.0, 1.0), (0.0, 1.0), rect);
        assert!((pos.x - 100.0).abs() < 1e-3);
        assert!((pos.y - 0.0).abs() < 1e-3); // y is inverted
    }
}
