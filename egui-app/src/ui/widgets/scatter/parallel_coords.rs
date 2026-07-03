use crate::theme::chart_colors::{
    COLOR_CHART_TEXT, COLOR_INFEASIBLE, COLOR_PARALLEL_AXIS, COLOR_PARALLEL_LINE_DEFAULT,
    COLOR_PARALLEL_LINE_UNSELECTED, COLOR_PARALLEL_TICK,
};
use crate::theme::CENTRAL_BG;
use crate::ui::widgets::common::range_math;
use crate::ui::widgets::scatter_matrix::downsample_indices_to_cap;

/// PCP 折れ線の最大描画数。1 試行あたり線分 (n_visible-1) 本を描くため
/// 散布図の点よりも描画コストが高く、scatter_matrix の `MAX_SCATTER_POINTS` と
/// 同じ上限を採用する（ブラシで選択中のトライアルは間引き対象外・常に描画する）。
const MAX_PCP_POLYLINES: usize = 1500;

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
    range_math::normalize01(v, v_min, v_max)
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

/// `axis_visibility` に基づき、描画対象（可視）軸の元インデックス一覧を返す。
/// 未登録の軸は表示扱い（`unwrap_or(true)`）とし、デフォルトでは全軸が可視になる。
pub fn visible_axis_indices(
    all_names: &[String],
    axis_visibility: &std::collections::HashMap<String, bool>,
) -> Vec<usize> {
    (0..all_names.len())
        .filter(|&i| axis_visibility.get(&all_names[i]).copied().unwrap_or(true))
        .collect()
}

/// 色付け用の正規化レンジを実行可能解のみから算出する。
/// 実行不可能解の外れ値でカラーマップが圧縮されないよう、軸の座標レンジとは別に求める。
/// 制約なし（feas.has_constraints() == false）の場合は全件、有効な値が一つも無い場合は `fallback` を返す。
pub fn feasible_color_range(
    col: &[f64],
    feas: tunny_core::dataframe::Feasibility<'_>,
    fallback: (f64, f64),
) -> (f64, f64) {
    let (mn, mx) = col
        .iter()
        .enumerate()
        .filter(|(idx, v)| v.is_finite() && feas.is_feasible(*idx))
        .map(|(_, &v)| v)
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(mn, mx), v| {
            (mn.min(v), mx.max(v))
        });
    if mn <= mx {
        (mn, mx)
    } else {
        fallback
    }
}

/// PCP ブラシ操作のドラッグ種別。
enum BrushDrag {
    /// 新規範囲の作成（従来挙動）。`drag_start` のアンカーから現在位置まで範囲を引く。
    Create,
    /// 既存範囲の平行移動。`grab_norm_y` はドラッグ開始時のポインタ正規化 Y、
    /// `orig_range` は移動前の `(lo, hi)`。差分を加算して範囲全体をスライドする。
    Move {
        grab_norm_y: f32,
        orig_range: (f32, f32),
    },
}

/// 既存ブラシ範囲を `delta`（正規化）だけ平行移動する。
/// 幅を保ったまま [0, 1] に収まるよう端でクランプする。
pub fn shifted_brush_range(orig: (f32, f32), delta: f32) -> (f32, f32) {
    let (lo, hi) = orig;
    let width = hi - lo;
    // 下端・上端のどちらが先に境界へ達するかで delta を制限する
    let clamped_delta = delta.clamp(-lo, 1.0 - hi);
    let new_lo = lo + clamped_delta;
    (new_lo, new_lo + width)
}

/// 平行座標図ウィジェット
pub struct ParallelCoordsChart {
    pub axis_order: Vec<String>,
    pub show_params: bool,
    pub show_objectives: bool,
    pub brush_ranges: std::collections::HashMap<String, Option<(f32, f32)>>,
    pub drag_start: Option<(String, f32)>,
    /// 進行中のドラッグ種別（新規作成 or 既存範囲の移動）。drag_start と併用する。
    brush_drag: Option<BrushDrag>,
    /// REQ-004: 軸ごとの表示/非表示フラグ（true = 表示）
    pub axis_visibility: std::collections::HashMap<String, bool>,
    col_ranges_cache: Option<Vec<(f64, f64)>>,
    cache_key: (usize, usize, usize), // (trial_count, n_params, n_objs)
    /// 折れ線描画の間引きインデックスキャッシュ（trial_count が変わらない限り再計算しない）
    polyline_indices_cache: Option<Vec<u32>>,
    polyline_indices_cache_key: Option<usize>, // trial_count
    /// 軸ラベルの事前レイアウト済み Galley キャッシュ（軸名リストが変わらない限り再計算しない）
    label_galleys_cache: Option<Vec<std::sync::Arc<egui::Galley>>>,
    label_galleys_cache_key: Option<Vec<String>>,
    // TASK-2242: pending selection from completed brush drag
    pub pending_selection: Option<Vec<u32>>,
    /// 実行不可能解を表示するか（制約あり Study でのみ有効）
    pub show_infeasible: bool,
    /// 線の色付けに使う軸名（None の場合は最後の軸 = 末尾の目的にフォールバック）
    pub color_axis: Option<String>,
}

impl Default for ParallelCoordsChart {
    fn default() -> Self {
        Self {
            axis_order: Vec::new(),
            show_params: true,
            show_objectives: true,
            brush_ranges: std::collections::HashMap::new(),
            drag_start: None,
            brush_drag: None,
            axis_visibility: std::collections::HashMap::new(),
            col_ranges_cache: None,
            cache_key: (0, 0, 0),
            polyline_indices_cache: None,
            polyline_indices_cache_key: None,
            label_galleys_cache: None,
            label_galleys_cache_key: None,
            pending_selection: None,
            show_infeasible: true,
            color_axis: None,
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
        self.brush_drag = None;
        self.pending_selection = None;
    }

    /// 平行座標プロットを描画する
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

        let feas = view.feasibility();
        let has_constraints = feas.has_constraints();

        // コントロール行: 描画軸の選択 + 色付け対象軸 + "Show Infeasible"
        ui.horizontal(|ui| {
            // 描画する軸を選ぶチェックボックス付きドロップダウン（デフォルト全表示）
            let visible_count = all_names
                .iter()
                .filter(|n| self.axis_visibility.get(*n).copied().unwrap_or(true))
                .count();
            ui.label("Axes:");
            egui::ComboBox::from_id_salt("pcp_visible_axes")
                .selected_text(format!("{visible_count}/{n_axes}"))
                .show_ui(ui, |ui| {
                    ui.horizontal(|ui| {
                        if ui.button("All").clicked() {
                            for name in &all_names {
                                self.axis_visibility.insert(name.clone(), true);
                            }
                        }
                        if ui.button("None").clicked() {
                            for name in &all_names {
                                self.axis_visibility.insert(name.clone(), false);
                            }
                        }
                    });
                    ui.separator();
                    for name in &all_names {
                        let mut vis = self.axis_visibility.get(name).copied().unwrap_or(true);
                        if ui.checkbox(&mut vis, name.as_str()).changed() {
                            self.axis_visibility.insert(name.clone(), vis);
                        }
                    }
                });

            // 線の色付け対象軸（未設定なら末尾の軸 = 末尾の目的）を解決する
            let current_axis = self
                .color_axis
                .clone()
                .filter(|name| all_names.iter().any(|n| n == name))
                .unwrap_or_else(|| all_names[n_axes - 1].clone());
            ui.label("Color by:");
            egui::ComboBox::from_id_salt("pcp_color_axis")
                .selected_text(current_axis.clone())
                .show_ui(ui, |ui| {
                    for name in &all_names {
                        if ui
                            .selectable_label(*name == current_axis, name.as_str())
                            .clicked()
                        {
                            self.color_axis = Some(name.clone());
                        }
                    }
                });
            if has_constraints {
                ui.checkbox(&mut self.show_infeasible, "Show Infeasible");
            }
        });

        // 描画対象（可視）軸の元インデックス一覧（未登録 = 表示）。
        let visible = visible_axis_indices(&all_names, &self.axis_visibility);
        let n_visible = visible.len();
        if n_visible < 2 {
            ui.centered_and_justified(|ui| {
                ui.label(egui::RichText::new("Select at least 2 axes to display.").weak());
            });
            return;
        }

        // 描画に使う色付け対象軸のインデックス（ドロップダウン反映後に解決）
        let color_axis_idx = self
            .color_axis
            .as_ref()
            .and_then(|name| all_names.iter().position(|n| n == name))
            .unwrap_or(n_axes - 1);

        let available = ui.available_rect_before_wrap();
        let axis_margin = 40.0_f32;
        let axis_x: Vec<f32> = (0..n_visible)
            .map(|i| {
                available.min.x
                    + axis_margin
                    + (available.width() - 2.0 * axis_margin) * i as f32 / (n_visible - 1) as f32
            })
            .collect();

        let painter = ui.painter().clone();
        let text_color = COLOR_CHART_TEXT;
        let label_font = egui::FontId::proportional(10.0);

        // 軸ラベルを事前レイアウトし、隣接軸より幅が広ければ斜めに回転させて重なりを防ぐ。
        // レイアウト（layout_no_wrap）はテキスト整形コストがあるため、軸名リストが
        // 変わらない限り毎フレーム再計算しない。
        if self.label_galleys_cache.is_none()
            || self.label_galleys_cache_key.as_deref() != Some(&all_names[..])
        {
            let galleys: Vec<std::sync::Arc<egui::Galley>> = all_names
                .iter()
                .map(|name| painter.layout_no_wrap(name.clone(), label_font.clone(), text_color))
                .collect();
            self.label_galleys_cache = Some(galleys);
            self.label_galleys_cache_key = Some(all_names.clone());
        }
        let label_galleys = self.label_galleys_cache.as_ref().unwrap();
        let max_label_w = visible
            .iter()
            .map(|&i| label_galleys[i].size().x)
            .fold(0.0_f32, f32::max);
        let label_h = label_galleys.first().map(|g| g.size().y).unwrap_or(12.0);
        let axis_spacing = (available.width() - 2.0 * axis_margin) / (n_visible - 1) as f32;
        let rotate_labels = max_label_w > axis_spacing - 4.0;
        let label_angle = if rotate_labels {
            std::f32::consts::FRAC_PI_4 // 45° 回転（右肩上がり）
        } else {
            0.0
        };
        // ラベルが占有する上端の高さ（回転時は対角方向の高さ）
        let label_area = if rotate_labels {
            (max_label_w * label_angle.sin() + label_h * label_angle.cos()).min(110.0) + 8.0
        } else {
            label_h + 8.0
        };
        let axis_top = available.min.y + label_area;
        let axis_bottom = available.max.y - 10.0;

        painter.rect_filled(available, 0.0, CENTRAL_BG);

        const N_TICKS: usize = 5;
        let tick_len = 4.0_f32;
        let tick_color = COLOR_PARALLEL_TICK;
        let tick_font = egui::FontId::proportional(9.0);

        let show_infeasible = self.show_infeasible;

        // 色付けの正規化レンジは実行可能解のみから算出する（軸の座標レンジとは別）。
        let color_range: (f64, f64) = match cols.get(color_axis_idx).and_then(|c| c.as_ref()) {
            Some(col) => feasible_color_range(col, feas, col_ranges[color_axis_idx]),
            None => col_ranges[color_axis_idx],
        };

        // アクティブなブラシが一つでもあれば、選択範囲外の線をグレーアウトする。
        // ドラッグ中も `brush_ranges` が更新されるためリアルタイムに反映される。
        let has_active_brush = self.brush_ranges.values().any(|range| range.is_some());

        // 描画対象トライアルの間引き: 全件描画は重いため MAX_PCP_POLYLINES 件に制限する。
        // trial_count が変わらない限り再計算しない。
        if self.polyline_indices_cache_key != Some(trial_count) {
            let all: Vec<u32> = (0..trial_count as u32).collect();
            self.polyline_indices_cache = Some(downsample_indices_to_cap(&all, MAX_PCP_POLYLINES));
            self.polyline_indices_cache_key = Some(trial_count);
        }
        let downsampled = self.polyline_indices_cache.as_ref().unwrap();

        // 描画対象 (t_idx, in_selection) の一覧。ブラシ選択中のトライアルは間引きの
        // 影響を受けず必ず描画する（間引き対象 ∪ ブラシ通過トライアルの和集合）。
        let draw_targets: Vec<(usize, bool)> = if has_active_brush {
            let downsampled_set: std::collections::HashSet<usize> =
                downsampled.iter().map(|&i| i as usize).collect();
            (0..trial_count)
                .filter_map(|t_idx| {
                    let passes = trial_passes_brushes(
                        t_idx,
                        &self.brush_ranges,
                        &cols,
                        col_ranges,
                        &all_names,
                    );
                    (downsampled_set.contains(&t_idx) || passes).then_some((t_idx, passes))
                })
                .collect()
        } else {
            downsampled.iter().map(|&i| (i as usize, true)).collect()
        };

        // 各試行を折れ線で描画（半透明）。
        // 選択外（グレーアウト）の線を先に描き、選択内の線を最前面に重ねる。
        // スクラッチバッファは即時描画分（非選択）で使い回し、per-trial のアロケーションを避ける。
        // 最前面に重ねる選択内の線だけは、後段でまとめて描くために個別に複製する。
        let mut selected_polylines: Vec<(Vec<egui::Pos2>, egui::Color32)> = Vec::new();
        let mut point_scratch: Vec<egui::Pos2> = Vec::with_capacity(n_visible);
        for (t_idx, in_selection) in draw_targets {
            let feasible = feas.is_feasible(t_idx);

            if !feasible && !show_infeasible {
                continue;
            }

            let color = if !in_selection {
                COLOR_PARALLEL_LINE_UNSELECTED
            } else if feasible {
                // 選択軸の値を [0,1] に正規化し、カラーマップで色を決める
                let base_color = cols
                    .get(color_axis_idx)
                    .and_then(|c| c.as_ref())
                    .and_then(|c| c.get(t_idx))
                    .copied()
                    .map(|v| {
                        let (mn, mx) = color_range;
                        cmap.interpolate(normalize_value(v, mn, mx))
                    })
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

            point_scratch.clear();
            let mut valid = true;
            for (disp, &orig) in visible.iter().enumerate() {
                let val_opt = cols
                    .get(orig)
                    .and_then(|c| c.as_ref())
                    .and_then(|c| c.get(t_idx))
                    .copied();
                let Some(val) = val_opt else {
                    valid = false;
                    break;
                };
                let (mn, mx) = col_ranges[orig];
                let norm = normalize_value(val, mn, mx);
                let y = normalized_to_screen_y(norm, axis_top, axis_bottom);
                point_scratch.push(egui::pos2(axis_x[disp], y));
            }
            if valid && point_scratch.len() >= 2 {
                if in_selection && has_active_brush {
                    // 選択内の線は後でまとめて最前面に描画する
                    selected_polylines.push((point_scratch.clone(), color));
                } else {
                    for pair in point_scratch.windows(2) {
                        painter.line_segment([pair[0], pair[1]], egui::Stroke::new(0.8, color));
                    }
                }
            }
        }
        // 選択内の線を最前面に重ねる
        for (points, color) in &selected_polylines {
            for pair in points.windows(2) {
                painter.line_segment([pair[0], pair[1]], egui::Stroke::new(0.8, *color));
            }
        }

        // 縦軸・ラベル・目盛りを最前面に描画
        for (disp, &orig) in visible.iter().enumerate() {
            let x = axis_x[disp];
            painter.line_segment(
                [egui::pos2(x, axis_top), egui::pos2(x, axis_bottom)],
                egui::Stroke::new(1.5, COLOR_PARALLEL_AXIS),
            );
            let galley = label_galleys[orig].clone();
            if rotate_labels {
                // -label_angle（反時計回り）で回転させた "/" 形ラベルの最下端
                // （= 文字列先頭・左下隅）を、各軸の上端 (x, axis_top) に合わせる。
                let size = galley.size();
                let applied = -label_angle;
                let (sa, ca) = (applied.sin(), applied.cos());
                // 4 隅を回転させ、画面上で最も下（最大 y）になる点を探す
                let corners = [(0.0, 0.0), (size.x, 0.0), (0.0, size.y), (size.x, size.y)];
                let mut lowest = (0.0_f32, f32::MIN); // pos からの相対オフセット (rx, ry)
                for (px, py) in corners {
                    let rx = px * ca - py * sa;
                    let ry = px * sa + py * ca;
                    if ry > lowest.1 {
                        lowest = (rx, ry);
                    }
                }
                // 最下端が軸上端のすぐ上に来るよう pos を決める
                let gap = 2.0_f32;
                let anchor = egui::pos2(x, axis_top - gap);
                let pos = anchor - egui::vec2(lowest.0, lowest.1);
                painter
                    .add(egui::epaint::TextShape::new(pos, galley, text_color).with_angle(applied));
            } else {
                let size = galley.size();
                painter.galley(
                    egui::pos2(x - size.x * 0.5, available.min.y + 4.0),
                    galley,
                    text_color,
                );
            }

            let (mn, mx) = col_ranges[orig];
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

        // Draw brush range overlays（可視軸のみ）
        for (disp, &orig) in visible.iter().enumerate() {
            let name = &all_names[orig];
            if let Some(Some((y_lo, y_hi))) = self.brush_ranges.get(name.as_str()) {
                let x = axis_x[disp];
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
                    egui::StrokeKind::Inside,
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
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|(i, _)| i);

            if let Some(disp_idx) = closest_axis_idx {
                let axis_name = all_names[visible[disp_idx]].clone();
                // Normalize pointer Y to [0, 1]
                let norm_y = ((axis_bottom - ptr.y) / (axis_bottom - axis_top)).clamp(0.0, 1.0);

                if response.drag_started() {
                    // 既存ブラシの内側をつかんだら移動モード、それ以外は新規作成モード。
                    let existing = self
                        .brush_ranges
                        .get(axis_name.as_str())
                        .and_then(|r| *r)
                        .filter(|(lo, hi)| norm_y >= *lo && norm_y <= *hi);
                    self.brush_drag = Some(match existing {
                        Some(orig_range) => BrushDrag::Move {
                            grab_norm_y: norm_y,
                            orig_range,
                        },
                        None => BrushDrag::Create,
                    });
                    self.drag_start = Some((axis_name, norm_y));
                } else if response.dragged() {
                    if let Some((ref start_name, start_y)) = self.drag_start.clone() {
                        if *start_name == axis_name {
                            let new_range = match self.brush_drag {
                                Some(BrushDrag::Move {
                                    grab_norm_y,
                                    orig_range,
                                }) => shifted_brush_range(orig_range, norm_y - grab_norm_y),
                                _ => ordered_brush_range(start_y, norm_y),
                            };
                            self.brush_ranges.insert(axis_name, Some(new_range));
                        }
                    }
                } else if response.drag_stopped() {
                    self.drag_start = None;
                    self.brush_drag = None;
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

        // 既存ブラシの矩形内をホバー中は「つかんで移動できる」ことを grab カーソルで示す
        if let Some(ptr) = response.hover_pos() {
            let hovering_brush = visible.iter().enumerate().any(|(disp, &orig)| {
                // ブラシ矩形は軸を中心に幅 ±6px（描画時と同じ）
                if (ptr.x - axis_x[disp]).abs() > 6.0 {
                    return false;
                }
                let name = &all_names[orig];
                self.brush_ranges
                    .get(name.as_str())
                    .and_then(|r| *r)
                    .map(|(lo, hi)| {
                        let norm_y =
                            ((axis_bottom - ptr.y) / (axis_bottom - axis_top)).clamp(0.0, 1.0);
                        norm_y >= lo && norm_y <= hi
                    })
                    .unwrap_or(false)
            });
            if hovering_brush {
                ui.ctx().set_cursor_icon(if response.dragged() {
                    egui::CursorIcon::Grabbing
                } else {
                    egui::CursorIcon::Grab
                });
            }
        }

        // Clear brushes on right-click or double-click
        if response.secondary_clicked() || response.double_clicked() {
            self.brush_ranges.clear();
            self.brush_drag = None;
            self.pending_selection = Some(vec![]); // empty = no selection filter
        }
    }
}

/// 単一トライアル（行インデックス `t_idx`）が全アクティブブラシ範囲を AND 条件で満たすか判定する。
/// 値が欠損している軸にアクティブブラシがある場合は不通過（false）とする。
pub fn trial_passes_brushes(
    t_idx: usize,
    brush_ranges: &std::collections::HashMap<String, Option<(f32, f32)>>,
    cols: &[Option<&[f64]>],
    col_ranges: &[(f64, f64)],
    all_names: &[String],
) -> bool {
    for (axis_idx, axis_name) in all_names.iter().enumerate() {
        let Some(Some((lo, hi))) = brush_ranges.get(axis_name.as_str()) else {
            continue; // no active brush on this axis
        };
        let Some(val) = cols
            .get(axis_idx)
            .and_then(|c| c.as_ref())
            .and_then(|c| c.get(t_idx))
            .copied()
        else {
            return false; // missing value but brush is active → excluded
        };
        let Some((mn, mx)) = col_ranges.get(axis_idx).copied() else {
            return false;
        };
        let norm = normalize_value(val, mn, mx);
        if norm < *lo || norm > *hi {
            return false; // outside brush range
        }
    }
    true
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
            if trial_passes_brushes(t_idx, brush_ranges, cols, col_ranges, all_names) {
                trial_ids.get(t_idx).copied()
            } else {
                None
            }
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
        assert!(chart.color_axis.is_none());
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
    fn shifted_brush_range_moves_within_bounds() {
        let (lo, hi) = shifted_brush_range((0.2, 0.5), 0.1);
        assert!((lo - 0.3).abs() < 1e-6);
        assert!((hi - 0.6).abs() < 1e-6);
    }

    #[test]
    fn shifted_brush_range_clamps_at_top() {
        // width 0.3, shift up by 0.4 → would exceed 1.0, clamp so hi == 1.0
        let (lo, hi) = shifted_brush_range((0.5, 0.8), 0.4);
        assert!((hi - 1.0).abs() < 1e-6);
        assert!((lo - 0.7).abs() < 1e-6); // width preserved
    }

    #[test]
    fn shifted_brush_range_clamps_at_bottom() {
        // shift down past 0 → clamp so lo == 0.0, width preserved
        let (lo, hi) = shifted_brush_range((0.2, 0.5), -0.4);
        assert!((lo - 0.0).abs() < 1e-6);
        assert!((hi - 0.3).abs() < 1e-6);
    }

    #[test]
    fn shifted_brush_range_preserves_width() {
        let orig = (0.1_f32, 0.6_f32);
        let (lo, hi) = shifted_brush_range(orig, 0.25);
        assert!(((hi - lo) - (orig.1 - orig.0)).abs() < 1e-6);
    }

    #[test]
    fn trial_passes_brushes_no_active_brush_passes() {
        use std::collections::HashMap;
        let col_data = [vec![2.0, 8.0], vec![5.0, 9.0]];
        let cols: Vec<Option<&[f64]>> =
            vec![Some(col_data[0].as_slice()), Some(col_data[1].as_slice())];
        let col_ranges = vec![(0.0_f64, 10.0_f64), (0.0_f64, 10.0_f64)];
        let all_names = vec!["x".to_string(), "obj".to_string()];
        // ブラシ未設定（None のみ）→ 全件通過
        let mut brush_ranges: HashMap<String, Option<(f32, f32)>> = HashMap::new();
        brush_ranges.insert("x".to_string(), None);
        assert!(trial_passes_brushes(
            0,
            &brush_ranges,
            &cols,
            &col_ranges,
            &all_names
        ));
    }

    #[test]
    fn trial_passes_brushes_missing_value_with_active_brush_excluded() {
        use std::collections::HashMap;
        // axis 1 は値が 1 件しかなく、t_idx=1 は欠損
        let col_data_x = vec![2.0, 8.0];
        let col_data_obj = vec![5.0];
        let cols: Vec<Option<&[f64]>> =
            vec![Some(col_data_x.as_slice()), Some(col_data_obj.as_slice())];
        let col_ranges = vec![(0.0_f64, 10.0_f64), (0.0_f64, 10.0_f64)];
        let all_names = vec!["x".to_string(), "obj".to_string()];
        let mut brush_ranges: HashMap<String, Option<(f32, f32)>> = HashMap::new();
        brush_ranges.insert("obj".to_string(), Some((0.0, 1.0)));
        // t_idx=1 は obj 値が欠損 → ブラシがアクティブなので不通過
        assert!(!trial_passes_brushes(
            1,
            &brush_ranges,
            &cols,
            &col_ranges,
            &all_names
        ));
    }

    #[test]
    fn visible_axis_indices_default_all_visible() {
        use std::collections::HashMap;
        let names = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let vis = HashMap::new(); // 未登録 = 全表示
        assert_eq!(visible_axis_indices(&names, &vis), vec![0, 1, 2]);
    }

    #[test]
    fn visible_axis_indices_filters_hidden_and_preserves_order() {
        use std::collections::HashMap;
        let names = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let mut vis = HashMap::new();
        vis.insert("b".to_string(), false);
        assert_eq!(visible_axis_indices(&names, &vis), vec![0, 2]);
    }

    #[test]
    fn visible_axis_indices_all_hidden_is_empty() {
        use std::collections::HashMap;
        let names = vec!["a".to_string(), "b".to_string()];
        let mut vis = HashMap::new();
        vis.insert("a".to_string(), false);
        vis.insert("b".to_string(), false);
        assert!(visible_axis_indices(&names, &vis).is_empty());
    }

    #[test]
    fn feasible_color_range_excludes_infeasible_outliers() {
        use tunny_core::dataframe::Feasibility;
        // 実行不可能解 (idx 3) が外れ値 1000.0 を持つが、レンジは実行可能解のみから算出する
        let col = [1.0, 2.0, 3.0, 1000.0];
        let feas_col = [1.0, 1.0, 1.0, 0.0];
        let feas = Feasibility::from_column(Some(&feas_col));
        let (mn, mx) = feasible_color_range(&col, feas, (0.0, 9999.0));
        assert_eq!(mn, 1.0);
        assert_eq!(mx, 3.0);
    }

    #[test]
    fn feasible_color_range_no_constraints_uses_all() {
        use tunny_core::dataframe::Feasibility;
        let col = [1.0, 2.0, 3.0, 1000.0];
        let feas = Feasibility::from_column(None);
        let (mn, mx) = feasible_color_range(&col, feas, (0.0, 9999.0));
        assert_eq!(mn, 1.0);
        assert_eq!(mx, 1000.0);
    }

    #[test]
    fn feasible_color_range_all_infeasible_falls_back() {
        use tunny_core::dataframe::Feasibility;
        let col = [1.0, 2.0, 3.0];
        let feas_col = [0.0, 0.0, 0.0];
        let feas = Feasibility::from_column(Some(&feas_col));
        let range = feasible_color_range(&col, feas, (-5.0, 5.0));
        assert_eq!(range, (-5.0, 5.0));
    }

    #[test]
    fn feasible_color_range_skips_non_finite() {
        use tunny_core::dataframe::Feasibility;
        let col = [1.0, f64::NAN, f64::INFINITY, 4.0];
        let feas_col = [1.0, 1.0, 1.0, 1.0];
        let feas = Feasibility::from_column(Some(&feas_col));
        let (mn, mx) = feasible_color_range(&col, feas, (0.0, 0.0));
        assert_eq!(mn, 1.0);
        assert_eq!(mx, 4.0);
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
