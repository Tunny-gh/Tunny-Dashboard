use crate::theme::chart_colors::{
    COLOR_CHART_TEXT, COLOR_GRID_STROKE, COLOR_INFEASIBLE, COLOR_SCATTER_DOT,
};
use crate::theme::color_compute::correlation_color;

/// 散布図セルあたりに描画する最大点数。これを超える試行は均等間引きで描画する。
/// セル数（下三角）×点数で描画コストが効くため、点数を抑えて応答性を保つ。
pub const MAX_SCATTER_POINTS: usize = 1500;

/// 対角セルのヒストグラムのビン数。
const HIST_BINS: usize = 10;

/// Scatter Matrix の表示モード
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum MatrixMode {
    ParamsVsParams,
    ParamsVsObjectives,
}

/// Scatter Matrix の軸ソート
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum AxisSort {
    Alphabetical,
    Correlation,
}

/// Scatter Matrix の全体状態
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct ScatterMatrix {
    pub mode: MatrixMode,
    pub sort: AxisSort,
    #[serde(skip)]
    pub selected_cell: Option<(usize, usize)>,
    /// 実行不可能解を表示するか（制約あり Study でのみ有効）
    pub show_infeasible: bool,
    /// 点の色付けに使う目的関数名（None は先頭の目的関数にフォールバック）
    pub color_objective: Option<String>,
    /// feasible/infeasible 分割＋間引き済みインデックスのキャッシュ（(feasible, infeasible)）
    #[serde(skip)]
    downsample_cache: Option<(Vec<u32>, Vec<u32>)>,
    #[serde(skip)]
    downsample_cache_key: Option<(usize, usize, bool)>, // (df_ptr, trial_count, has_constraints)
    /// セル統計（列レンジ・ヒストグラム・相関）と点色のキャッシュ（H-4）。
    #[serde(skip)]
    stats_cache: Option<MatrixStatsCache>,
    /// 行・列ラベルの事前レイアウト済み Galley キャッシュ（軸名リストが変わらない限り再計算しない）
    #[serde(skip)]
    label_galleys_cache: Option<Vec<std::sync::Arc<egui::Galley>>>,
    #[serde(skip)]
    label_galleys_cache_key: Option<Vec<String>>,
}

/// セル統計（列レンジ・ヒストグラム・相関）と点色のキャッシュ（H-4）。
///
/// セル描画ループは毎フレーム、全列・全 trial に対してヒストグラム・相関係数・min/max を
/// 再計算していた（O(n_axes² × trial_count)）。数万 trial × 十数パラメータではフレーム落ちの
/// 主要因になる。df の恒等性・表示モード・着色目的・カラーマップが変わらない限り、これらを
/// 一度だけ計算してキャッシュし、フレーム内は描画のみにする。
struct MatrixStatsCache {
    key: MatrixStatsKey,
    /// 各列の min/max（散布図セルの座標変換に使う。`draw_scatter_cell` と同じ畳み込み）。
    col_ranges: Vec<(f64, f64)>,
    /// 対角セルのヒストグラム（列ごと・`HIST_BINS` ビン）。
    histograms: Vec<Vec<usize>>,
    /// 上三角セルの相関係数。`row * n + col`（row < col のみ有効）でアクセスする。
    correlations: Vec<f64>,
    /// feasible 描画点の色（`feasible_draw` と同じ並び順）。
    point_colors: Vec<egui::Color32>,
}

/// `MatrixStatsCache` の無効化キー。いずれかが変われば全セル統計を再計算する。
#[derive(PartialEq)]
struct MatrixStatsKey {
    /// DataFrame の Arc 恒等性（別 Study/更新後の取り違えを防ぐ）。
    df_ptr: usize,
    mode: MatrixMode,
    color_objective: Option<String>,
    cmap_fp: u64,
    trial_count: usize,
    has_constraints: bool,
}

impl Default for ScatterMatrix {
    fn default() -> Self {
        Self {
            mode: MatrixMode::ParamsVsParams,
            sort: AxisSort::Alphabetical,
            selected_cell: None,
            show_infeasible: true,
            color_objective: None,
            downsample_cache: None,
            downsample_cache_key: None,
            stats_cache: None,
            label_galleys_cache: None,
            label_galleys_cache_key: None,
        }
    }
}

impl ScatterMatrix {
    /// 散布図行列を描画する
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        view: &crate::state::app_state::StudyView,
        param_names: &[String],
        obj_names: &[String],
        cmap: &crate::theme::colormap::ColorMap,
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

        let feas = view.feasibility();
        let has_constraints = feas.has_constraints();
        // DataFrame の Arc 恒等性。別 Study / 更新後の取り違えを防ぐキャッシュキーに使う。
        let df_ptr = std::sync::Arc::as_ptr(&view.df) as usize;

        // コントロール行: "Show Infeasible" トグル（制約あり Study のみ）と "Color by" ドロップダウン
        ui.horizontal(|ui| {
            if has_constraints {
                ui.checkbox(&mut self.show_infeasible, "Show Infeasible");
            }
            if !obj_names.is_empty() {
                // 解決済みの色付け対象目的関数名
                let current_obj =
                    resolve_color_objective(&self.color_objective, obj_names).unwrap_or("");
                ui.label("Color by:");
                egui::ComboBox::from_id_salt("scatter_matrix_color_obj")
                    .selected_text(current_obj)
                    .show_ui(ui, |ui| {
                        for name in obj_names {
                            if ui
                                .selectable_label(*name == current_obj, name.as_str())
                                .clicked()
                            {
                                self.color_objective = Some(name.clone());
                            }
                        }
                    });
            }
        });

        let show_infeasible = self.show_infeasible;

        // 描画パフォーマンス対策: セルあたりの表示点数に上限を設ける。
        // 全散布図セルで同じ間引きインデックスを使い回す（毎セル再計算しない）。
        // feasible/infeasible 分割＋間引きは trial_count・制約有無が変わらない限り再計算しない。
        let ds_key = (df_ptr, trial_count, has_constraints);
        if self.downsample_cache.is_none() || self.downsample_cache_key != Some(ds_key) {
            let (feasible_indices, infeasible_indices) =
                split_feasibility_indices(trial_count, feas);
            let feasible_draw = downsample_indices_to_cap(&feasible_indices, MAX_SCATTER_POINTS);
            let infeasible_draw =
                downsample_indices_to_cap(&infeasible_indices, MAX_SCATTER_POINTS);
            self.downsample_cache = Some((feasible_draw, infeasible_draw));
            self.downsample_cache_key = Some(ds_key);
        }

        // セル統計（列レンジ・ヒストグラム・相関）と点色を、df の恒等性・表示モード・
        // 着色目的・カラーマップをキーに一度だけ計算してキャッシュする（H-4）。
        // これらが変わらない限り、以降のセル描画ループは描画のみになる。
        let stats_key = MatrixStatsKey {
            df_ptr,
            mode: self.mode.clone(),
            color_objective: self.color_objective.clone(),
            cmap_fp: super::rank_plot::cmap_fingerprint(cmap),
            trial_count,
            has_constraints,
        };
        if self.stats_cache.as_ref().map(|c| &c.key) != Some(&stats_key) {
            // 列レンジ（`draw_scatter_cell` と同じ畳み込みで min/max を求める）。
            let col_ranges: Vec<(f64, f64)> = cols.iter().map(|c| col_min_max(c)).collect();
            // 対角セルのヒストグラム。
            let histograms: Vec<Vec<usize>> =
                cols.iter().map(|c| compute_histogram(c, HIST_BINS)).collect();
            // 上三角セルの相関係数（row < col のみ計算）。
            let mut correlations = vec![0.0f64; n * n];
            for row in 0..n {
                for col in (row + 1)..n {
                    correlations[row * n + col] = compute_correlation(cols[row], cols[col]);
                }
            }
            // feasible 描画点の色（間引き後の点数分のみ）。
            let point_colors = {
                let feasible_draw = &self.downsample_cache.as_ref().unwrap().0;
                compute_feasible_point_colors(
                    view,
                    &self.color_objective,
                    obj_names,
                    feas,
                    cmap,
                    feasible_draw,
                )
            };
            self.stats_cache = Some(MatrixStatsCache {
                key: stats_key,
                col_ranges,
                histograms,
                correlations,
                point_colors,
            });
        }
        let stats = self.stats_cache.as_ref().unwrap();
        let (feasible_draw, infeasible_draw) = self.downsample_cache.as_ref().unwrap();

        // 行・列ラベルを事前レイアウトしてサイズを測る。
        // レイアウトは軸名リストが変わらない限り毎フレーム再計算しない。
        let outer = ui.available_rect_before_wrap();
        let painter = ui.painter().clone();
        let label_color = ui.visuals().text_color();
        let label_font = egui::FontId::proportional(10.0);
        if self.label_galleys_cache.is_none()
            || self.label_galleys_cache_key.as_deref() != Some(&all_names[..])
        {
            let galleys: Vec<std::sync::Arc<egui::Galley>> = all_names
                .iter()
                .map(|name| painter.layout_no_wrap(name.clone(), label_font.clone(), label_color))
                .collect();
            self.label_galleys_cache = Some(galleys);
            self.label_galleys_cache_key = Some(all_names.clone());
        }
        let label_galleys = self.label_galleys_cache.as_ref().unwrap();
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
                // 各列中心・グリッド上端のすぐ上に合わせる（PCP と同じ手法・D-12 共通ヘルパー）
                let applied = -label_angle;
                let lowest = super::rotated_label_corners(size, applied).lowest;
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
                // 各行中心・グリッド左端のすぐ左に合わせる（D-12 共通ヘルパー）。
                let applied = -label_angle;
                let corners = super::rotated_label_corners(size, applied);
                let right = corners.rightmost;
                let (min_ry, max_ry) = corners.ry_range;
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
        // 点色はキャッシュ済み（stats.point_colors）。実際に描画するのは間引き後の
        // feasible_draw/infeasible_draw のみで、色配列も間引き後の点数分だけ持つ
        // （draw_scatter_cell は colors を downsample_indices と同じ並び順で参照する）。
        // infeasible は単色のため、テーマ色の変化に追従できるよう毎フレーム安価に構築する。
        let infeasible_colors: Vec<egui::Color32> = vec![COLOR_INFEASIBLE(); infeasible_draw.len()];

        for row in 0..n {
            for col in 0..n {
                let min = available.min + egui::vec2(col as f32 * cell_w, row as f32 * cell_h);
                let cell_rect = egui::Rect::from_min_size(min, egui::vec2(cell_w, cell_h));

                if row == col {
                    draw_histogram_bars(&painter, cell_rect, &stats.histograms[row]);
                } else if col > row {
                    // 上三角: 相関係数（キャッシュ済みの値を描画）
                    draw_correlation_cell(&painter, cell_rect, stats.correlations[row * n + col]);
                } else {
                    // 下三角: 散布図（間引き済みインデックス + キャッシュ済み列レンジで描画）
                    if has_constraints && show_infeasible && !infeasible_draw.is_empty() {
                        // infeasible を背面に描画
                        draw_scatter_cell(
                            &painter,
                            cell_rect,
                            cols[col],
                            cols[row],
                            stats.col_ranges[col],
                            stats.col_ranges[row],
                            &infeasible_colors,
                            Some(infeasible_draw),
                        );
                    }
                    // feasible（制約なし時は全点）を前面に描画
                    draw_scatter_cell(
                        &painter,
                        cell_rect,
                        cols[col],
                        cols[row],
                        stats.col_ranges[col],
                        stats.col_ranges[row],
                        &stats.point_colors,
                        Some(feasible_draw),
                    );
                }

                // 各セルに枠線を描画してセル境界を明示する
                // （高密度の散布図でも図の範囲が分かるように）
                painter.rect_stroke(
                    cell_rect,
                    0.0,
                    egui::Stroke::new(1.0, COLOR_GRID_STROKE()),
                    egui::StrokeKind::Inside,
                );
            }
        }

        ui.allocate_rect(outer, egui::Sense::hover());
    }
}

/// 色付け対象の目的関数名を解決する純関数。
/// - `selected` が `obj_names` に含まれる場合はその名前を返す。
/// - None または存在しない名前の場合は先頭要素（`obj_names[0]`）を返す。
/// - `obj_names` が空の場合は `None` を返す。
pub fn resolve_color_objective<'a>(
    selected: &Option<String>,
    obj_names: &'a [String],
) -> Option<&'a str> {
    if obj_names.is_empty() {
        return None;
    }
    if let Some(name) = selected {
        if let Some(found) = obj_names.iter().find(|n| *n == name) {
            return Some(found.as_str());
        }
    }
    Some(obj_names[0].as_str())
}

/// feasibility から feasible / infeasible インデックスリストを構築する。
/// 制約なし Study（feas.has_constraints() == false）の場合は全件を feasible 扱いとする。
pub fn split_feasibility_indices(
    n: usize,
    feas: tunny_core::dataframe::Feasibility<'_>,
) -> (Vec<u32>, Vec<u32>) {
    let (f_idx, inf_idx) = feas.partition_indices(n);
    let feasible: Vec<u32> = f_idx.into_iter().map(|i| i as u32).collect();
    let infeasible: Vec<u32> = inf_idx.into_iter().map(|i| i as u32).collect();
    (feasible, infeasible)
}

/// インデックス列を最大 `cap` 件まで均等間引きする。
/// `cap` 以下ならそのまま複製、超える場合は等間隔ストライドで間引いて
/// 全体の分布形状を保ったまま点数を減らす。
pub fn downsample_indices_to_cap(indices: &[u32], cap: usize) -> Vec<u32> {
    if cap == 0 {
        return Vec::new();
    }
    if indices.len() <= cap {
        return indices.to_vec();
    }
    // ストライドは切り上げ気味に取り、結果が cap を超えないようにする
    let step = indices.len().div_ceil(cap);
    indices.iter().step_by(step).copied().collect()
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

/// Pearson 相関係数を計算する。
///
/// 計算ロジックは `tunny_core::math::stats::pearson_correlation` に委譲する。
/// ただしセル表示用に、退化ケース（要素数 < 2 や分散がほぼ 0）では NaN では
/// なく 0.0 を返し、浮動小数点誤差に備えて結果を [-1, 1] にクランプする。
pub fn compute_correlation(x: &[f64], y: &[f64]) -> f64 {
    let n = x.len().min(y.len());
    let r = tunny_core::math::stats::pearson_correlation(&x[..n], &y[..n]);
    if r.is_nan() {
        0.0
    } else {
        r.clamp(-1.0, 1.0)
    }
}

/// 列データの min/max を返す（散布図セルの座標変換用）。
/// `f64::min`/`f64::max` の畳み込みは NaN を無視し、Inf は反映する（従来挙動を維持）。
pub fn col_min_max(data: &[f64]) -> (f64, f64) {
    let mn = data.iter().cloned().fold(f64::INFINITY, f64::min);
    let mx = data.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    (mn, mx)
}

/// 散布図行列の feasible 描画点の色（`feasible_draw` と同じ並び順）を計算する。
/// 目的関数がない・列が取れない場合は全点 `COLOR_SCATTER_DOT`。
/// 実際に描画するのは間引き後の点のみのため、色配列も間引き後の点数分だけ計算する。
fn compute_feasible_point_colors(
    view: &crate::state::app_state::StudyView,
    color_objective: &Option<String>,
    obj_names: &[String],
    feas: tunny_core::dataframe::Feasibility<'_>,
    cmap: &crate::theme::colormap::ColorMap,
    feasible_draw: &[u32],
) -> Vec<egui::Color32> {
    use super::parallel_coords::{feasible_color_range, normalize_value};
    let Some(name) = resolve_color_objective(color_objective, obj_names) else {
        return vec![COLOR_SCATTER_DOT(); feasible_draw.len()];
    };
    let Some(col) = view.numeric_column(name) else {
        return vec![COLOR_SCATTER_DOT(); feasible_draw.len()];
    };
    let (col_min, col_max) = col_min_max(col);
    let (mn, mx) = feasible_color_range(col, feas, (col_min, col_max));
    feasible_draw
        .iter()
        .map(|&i| {
            let v = col.get(i as usize).copied().unwrap_or(f64::NAN);
            if v.is_finite() {
                cmap.interpolate(normalize_value(v, mn, mx))
            } else {
                COLOR_SCATTER_DOT()
            }
        })
        .collect()
}

/// 散布図セルを painter で描画する。
/// `colors` はトライアル全体分ではなく、実際に描画するインデックス列（`downsample_indices`
/// があればその並び順、無ければ 0..x_data.len()）に対応する分だけ渡せばよい
/// （呼び出し側で間引き後の点数だけ計算することでフレームごとの計算量を抑える）。
/// `x_range`/`y_range` は列の min/max を事前計算して渡す（毎フレームの畳み込みを避ける・H-4）。
#[allow(clippy::too_many_arguments)]
pub fn draw_scatter_cell(
    painter: &egui::Painter,
    cell_rect: egui::Rect,
    x_data: &[f64],
    y_data: &[f64],
    x_range: (f64, f64),
    y_range: (f64, f64),
    colors: &[egui::Color32],
    downsample_indices: Option<&[u32]>,
) {
    let (x_min, x_max) = x_range;
    let (y_min, y_max) = y_range;

    let indices: Box<dyn Iterator<Item = usize>> = if let Some(ds) = downsample_indices {
        Box::new(ds.iter().map(|&i| i as usize))
    } else {
        Box::new(0..x_data.len())
    };

    for (k, i) in indices.enumerate() {
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
        let color = colors.get(k).copied().unwrap_or(COLOR_SCATTER_DOT());
        painter.circle_filled(pos, 1.6, color);
    }
}

/// 事前計算済みヒストグラムビンを painter で棒グラフとして描画する。
/// ビンの計算（`compute_histogram`）は呼び出し側でキャッシュする（H-4）。
pub fn draw_histogram_bars(painter: &egui::Painter, cell_rect: egui::Rect, bins: &[usize]) {
    let n_bins = bins.len().max(1);
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
        painter.rect_filled(bar_rect, 0.0, COLOR_SCATTER_DOT());
    }
}

/// 事前計算済みの相関係数 `corr` を painter でセルとして描画する。
/// 相関の計算（`compute_correlation`）は呼び出し側でキャッシュする（H-4）。
pub fn draw_correlation_cell(painter: &egui::Painter, cell_rect: egui::Rect, corr: f64) {
    let bg_color = correlation_color(corr);
    painter.rect_filled(cell_rect, 0.0, bg_color);
    painter.text(
        cell_rect.center(),
        egui::Align2::CENTER_CENTER,
        format!("{:.2}", corr),
        egui::FontId::proportional(12.0),
        COLOR_CHART_TEXT(),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── resolve_color_objective ──────────────────────────────────────

    #[test]
    fn resolve_color_objective_none_returns_first() {
        let names = vec!["obj0".to_string(), "obj1".to_string()];
        assert_eq!(resolve_color_objective(&None, &names), Some("obj0"));
    }

    #[test]
    fn resolve_color_objective_existing_name_returns_it() {
        let names = vec!["obj0".to_string(), "obj1".to_string()];
        assert_eq!(
            resolve_color_objective(&Some("obj1".to_string()), &names),
            Some("obj1")
        );
    }

    #[test]
    fn resolve_color_objective_unknown_name_falls_back_to_first() {
        let names = vec!["obj0".to_string(), "obj1".to_string()];
        assert_eq!(
            resolve_color_objective(&Some("unknown".to_string()), &names),
            Some("obj0")
        );
    }

    #[test]
    fn resolve_color_objective_empty_names_returns_none() {
        assert_eq!(resolve_color_objective(&None, &[]), None);
        assert_eq!(
            resolve_color_objective(&Some("obj0".to_string()), &[]),
            None
        );
    }

    // ── constraint-aware visualization (TASK-2350) ──────────────────

    #[test]
    fn tc_cav_scatter_matrix_show_infeasible_default_true() {
        let sm = ScatterMatrix::default();
        assert!(sm.show_infeasible);
    }

    #[test]
    fn tc_cav_split_feasibility_no_constraints_all_feasible() {
        use tunny_core::dataframe::Feasibility;
        let feas = Feasibility::from_column(None);
        let (f, inf) = split_feasibility_indices(3, feas);
        assert_eq!(f, vec![0, 1, 2]);
        assert!(inf.is_empty());
    }

    #[test]
    fn tc_cav_split_feasibility_mixed() {
        use tunny_core::dataframe::Feasibility;
        let col = vec![1.0_f64, 0.0, 1.0];
        let feas = Feasibility::from_column(Some(&col));
        let (f, inf) = split_feasibility_indices(3, feas);
        assert_eq!(f, vec![0, 2]);
        assert_eq!(inf, vec![1]);
    }

    #[test]
    fn tc_cav_split_feasibility_all_infeasible() {
        use tunny_core::dataframe::Feasibility;
        let col = vec![0.0_f64, 0.0];
        let feas = Feasibility::from_column(Some(&col));
        let (f, inf) = split_feasibility_indices(2, feas);
        assert!(f.is_empty());
        assert_eq!(inf, vec![0, 1]);
    }

    // TASK-2019 tests

    #[test]
    fn scatter_matrix_default_mode() {
        let sm = ScatterMatrix::default();
        assert_eq!(sm.mode, MatrixMode::ParamsVsParams);
        assert_eq!(sm.sort, AxisSort::Alphabetical);
        assert!(sm.selected_cell.is_none());
    }

    #[test]
    fn downsample_cap_keeps_all_when_under_cap() {
        let idx: Vec<u32> = (0..100).collect();
        let out = downsample_indices_to_cap(&idx, 4000);
        assert_eq!(out, idx);
    }

    #[test]
    fn downsample_cap_limits_when_over_cap() {
        let idx: Vec<u32> = (0..100_000).collect();
        let out = downsample_indices_to_cap(&idx, 4000);
        assert!(out.len() <= 4000, "got {}", out.len());
        assert!(!out.is_empty());
        // 先頭は保持され、間引きは昇順を維持する
        assert_eq!(out[0], 0);
        assert!(out.windows(2).all(|w| w[0] < w[1]));
    }

    #[test]
    fn downsample_cap_zero_is_empty() {
        let idx: Vec<u32> = (0..10).collect();
        assert!(downsample_indices_to_cap(&idx, 0).is_empty());
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
