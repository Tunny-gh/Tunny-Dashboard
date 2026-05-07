use crate::theme::chart_colors::{COLOR_CHART_TEXT, COLOR_SCATTER_DOT};
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
    col_data_cache: Option<Vec<Vec<f64>>>,
    cache_key: (usize, usize, usize), // (trial_count, n_params, n_objs)
}

impl ScatterMatrix {
    pub fn new() -> Self {
        Self {
            mode: MatrixMode::ParamsVsParams,
            sort: AxisSort::Alphabetical,
            selected_cell: None,
            col_data_cache: None,
            cache_key: (0, 0, 0),
        }
    }

    /// 散布図行列を描画する
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

        let all_names: Vec<String> = param_names
            .iter()
            .chain(obj_names.iter())
            .cloned()
            .collect();
        let n = all_names.len();
        if n == 0 {
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
            self.col_data_cache = Some(col_data);
            self.cache_key = cache_key;
        }
        let col_data = self.col_data_cache.as_ref().unwrap();

        let available = ui.available_rect_before_wrap();
        let cell_w = available.width() / n as f32;
        let cell_h = available.height() / n as f32;
        let painter = ui.painter().clone();
        let dot_color = COLOR_SCATTER_DOT;
        let point_colors: Vec<egui::Color32> = if chart_colors.is_empty() {
            vec![dot_color; trial_rows.len()]
        } else {
            chart_colors.to_vec()
        };

        for row in 0..n {
            for col in 0..n {
                let min = available.min + egui::vec2(col as f32 * cell_w, row as f32 * cell_h);
                let cell_rect = egui::Rect::from_min_size(min, egui::vec2(cell_w, cell_h));

                if row == col {
                    draw_histogram_cell(&painter, cell_rect, &col_data[row], 10);
                } else if col > row {
                    // 上三角: 相関係数
                    draw_correlation_cell(&painter, cell_rect, &col_data[row], &col_data[col]);
                } else {
                    // 下三角: 散布図
                    draw_scatter_cell(
                        &painter,
                        cell_rect,
                        &col_data[col],
                        &col_data[row],
                        &point_colors,
                        None,
                    );
                }
            }
        }

        ui.allocate_rect(available, egui::Sense::hover());
    }
}

impl Default for ScatterMatrix {
    fn default() -> Self {
        Self::new()
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
        painter.circle_filled(pos, 2.0, color);
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
        assert!(corr >= -1.0 && corr <= 1.0);
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
