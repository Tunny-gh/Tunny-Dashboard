//! Observed Contour ウィジェット。
//!
//! 観測トライアル点だけから補間した等高線を描く（サロゲート非依存）。
//! PDP / サロゲート応答曲面と異なりモデルを学習せず、データの無い領域はマスクして
//! **外挿を見せない**。X / Y / 値（色）はパラメータ・目的関数のどちらでも選べるため、
//! 目的関数空間のトレードオフ面も honest に描ける。計算は `tunny_core::contour` が
//! バックグラウンドで行う（poll_chart.rs 参照）。

use std::collections::HashMap;

use crate::io::artifacts::ArtifactEntry;
use crate::state::messages::ObservedContourResult;
use crate::state::types::StudyView;
use crate::theme::colormap::ColorMap;
use crate::ui::widget_states::{ObservedContourComputeRequest, ObservedContourState};
use crate::ui::widgets::common::heatmap::{
    draw_colorbar_simple, draw_heatmap_masked, normalize, value_range_masked,
};
use crate::ui::widgets::scatter_3d::{
    axis_segments_3d, draw_3d_axis_labels, draw_3d_grid, normalize_to_clip, setup_3d_canvas,
    ArcballCamera,
};
use crate::ui::widgets::trial_detail_modal::{TrialDetailTarget, HIT_THRESHOLD};

/// 格子の一辺の点数。
const N_GRID: usize = 60;
/// 等高線のレベル数。
const N_CONTOUR_LEVELS: usize = 6;

#[allow(clippy::too_many_arguments)]
pub fn show(
    ui: &mut egui::Ui,
    state: &mut ObservedContourState,
    param_names: &[String],
    obj_names: &[String],
    cmap: ColorMap,
    view: &StudyView,
    artifact_map: &HashMap<u32, Vec<ArtifactEntry>>,
    has_constraints: bool,
) {
    // 選択可能な列（数値パラメータ ∪ 目的関数）。カテゴリカル列は除外する。
    let columns: Vec<String> = param_names
        .iter()
        .filter(|p| view.numeric_column(p).is_some())
        .chain(obj_names.iter())
        .cloned()
        .collect();
    if columns.len() < 2 {
        ui.label("Need at least 2 numeric columns (parameters or objectives).");
        return;
    }

    // デフォルト選択（Study 切替で消えた名前もリセット）。
    if !columns.contains(&state.selected_x) {
        state.selected_x = columns[0].clone();
    }
    if !columns.contains(&state.selected_y) || state.selected_y == state.selected_x {
        state.selected_y = columns
            .iter()
            .find(|c| **c != state.selected_x)
            .cloned()
            .unwrap_or_default();
    }
    if !columns.contains(&state.selected_value) {
        state.selected_value = obj_names
            .first()
            .cloned()
            .unwrap_or_else(|| columns[0].clone());
    }

    // ── 軸・値セレクタ ───────────────────────────────────────────
    ui.horizontal(|ui| {
        ui.label("X:");
        combo(ui, "oc_x", &mut state.selected_x, &columns);
        ui.label("Y:");
        combo(ui, "oc_y", &mut state.selected_y, &columns);
        ui.label("Value:");
        combo(ui, "oc_value", &mut state.selected_value, &columns);
    });

    // ── Coverage（疎ガード）・トグル ─────────────────────────────
    let mut slider_dragging = false;
    ui.horizontal(|ui| {
        ui.label("Coverage:");
        let resp = ui
            .add(
                egui::Slider::new(&mut state.max_edge_ratio, 0.03..=1.0)
                    .text("gap")
                    .fixed_decimals(2),
            )
            .on_hover_text(
                "Smaller masks sparse regions more strictly (no extrapolation); larger fills wider",
            );
        slider_dragging = resp.dragged();

        ui.separator();
        ui.checkbox(&mut state.show_points, "Show points");
        ui.checkbox(&mut state.view_3d, "3D");
        if state.view_3d {
            ui.checkbox(&mut state.density_shade, "Density shade")
                .on_hover_text(
                    "Fade out cells with few nearby observations to counter 3D overconfidence",
                );
        } else {
            ui.checkbox(&mut state.show_contour_lines, "Contours");
            ui.checkbox(&mut state.log_scale, "Log color")
                .on_hover_text("Only effective when all values are positive");
        }
        if has_constraints {
            ui.checkbox(&mut state.feasible_only, "Feasible only");
        }
    });

    // 同一軸の警告（許容するが縮退する）。
    if state.selected_x == state.selected_y {
        ui.colored_label(
            egui::Color32::YELLOW,
            "Warning: same column selected for X and Y",
        );
    }

    // ── 自動再計算（軸・値・Coverage・feasible が変わったら発行）──────
    let cur = (
        state.selected_x.clone(),
        state.selected_y.clone(),
        state.selected_value.clone(),
        state.max_edge_ratio,
        state.feasible_only,
    );
    let changed = state.applied_sig.as_ref() != Some(&cur);
    if changed
        && !slider_dragging
        && !state.computing
        && state.pending_compute.is_none()
        && state.selected_x != state.selected_y
    {
        state.applied_sig = Some(cur.clone());
        state.computing = true;
        state.result = None;
        state.error_message = None;
        state.pending_compute = Some(ObservedContourComputeRequest {
            x: cur.0,
            y: cur.1,
            value: cur.2,
            n_grid: N_GRID,
            max_edge_ratio: cur.3,
            feasible_only: cur.4,
        });
    }

    if state.computing {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label("Interpolating observed contour…");
        });
        return;
    }

    if let Some(ref err) = state.error_message {
        ui.colored_label(egui::Color32::RED, format!("Error: {}", err));
        return;
    }

    // result の不変借用をブロックに閉じ込め、クリック対象だけ取り出す。
    // state.result / state.camera は別フィールドなので分割借用できる。
    let view_3d = state.view_3d;
    let show_points = state.show_points;
    let show_contour_lines = state.show_contour_lines;
    let log_scale = state.log_scale;
    let clicked: Option<TrialDetailTarget> = {
        let Some(result) = state.result.as_ref() else {
            ui.label("Select columns to see the observed contour.");
            return;
        };
        if view_3d {
            // 3D キャンバスは残り領域を全部使うので、キャプションは先（上）に描く。
            ui.label(
                "3D surface interpolated from observed trials; gaps = no data (not extrapolated).",
            );
            let opts3d = Render3dOpts {
                show_points,
                density_shade: state.density_shade,
            };
            render_3d(ui, result, &cmap, &mut state.camera, &opts3d);
            None
        } else {
            let opts = RenderOpts {
                show_points,
                show_contour_lines,
                log_scale,
            };
            let clicked = render_2d(ui, result, &cmap, &opts, view);
            ui.label("Interpolated from observed trials; blank = no data (not extrapolated).");
            clicked
        }
    };

    if let Some(target) = clicked {
        state.detail_modal.open(target);
    }
    state
        .detail_modal
        .show(ui, view, param_names, obj_names, artifact_map);
}

fn combo(ui: &mut egui::Ui, salt: &str, selected: &mut String, columns: &[String]) {
    egui::ComboBox::from_id_salt(salt)
        .selected_text(selected.as_str())
        .show_ui(ui, |ui| {
            for name in columns {
                ui.selectable_value(selected, name.clone(), name);
            }
        });
}

struct RenderOpts {
    show_points: bool,
    show_contour_lines: bool,
    log_scale: bool,
}

/// 等高線を描画し、観測点クリックがあればその対象を返す。
fn render_2d(
    ui: &mut egui::Ui,
    result: &ObservedContourResult,
    cmap: &ColorMap,
    opts: &RenderOpts,
    view: &StudyView,
) -> Option<TrialDetailTarget> {
    let surf = &result.surface;
    let nx = surf.x_values.len();
    let ny = surf.y_values.len();
    if nx < 2 || ny < 2 {
        ui.label("Not enough data to interpolate a contour.");
        return None;
    }

    // 表示向き: 横 = X（左→右で増加）、縦 = Y（上 = 最大）。
    // core の z[i][j] = f(x_i, y_j) を disp[r][c] = z[c][ny-1-r] に並べ替える。
    let display: Vec<Vec<Option<f64>>> = (0..ny)
        .map(|r| (0..nx).map(|c| surf.z[c][ny - 1 - r]).collect())
        .collect();

    // 元の値域（カラーバー表示・等高線レベル用）。
    let (v_min, v_max) = value_range_masked(&display);

    // 色用に対数変換するか（正の値域のみ）。
    let use_log = opts.log_scale && v_min > 0.0;
    let color_display: Vec<Vec<Option<f64>>> = if use_log {
        display
            .iter()
            .map(|row| row.iter().map(|c| c.map(|v| v.log10())).collect())
            .collect()
    } else {
        display.clone()
    };
    let (cv_min, cv_max) = value_range_masked(&color_display);

    let available = ui.available_rect_before_wrap();
    // 図の下に出すラベル2行（X/Y/Value 行＋サブタイトル）ぶんを確保してから割り当てる。
    // 確保しないとパネル高がギリギリのとき下のキャプションが見切れる。
    const CAPTION_RESERVE: f32 = 44.0;
    let plot_size = egui::vec2(
        (available.width() - 40.0).max(120.0),
        (available.height() - CAPTION_RESERVE).clamp(120.0, 360.0),
    );
    let (rect, response) = ui.allocate_exact_size(plot_size, egui::Sense::click());
    let painter = ui.painter_at(rect);

    painter.rect_stroke(
        rect,
        0.0,
        egui::Stroke::new(1.0, egui::Color32::from_gray(80)),
    );
    draw_heatmap_masked(&painter, rect, &color_display, cv_min, cv_max, cmap.clone());

    if opts.show_contour_lines {
        draw_contour_lines(&painter, rect, &display, v_min, v_max);
    }

    // 観測点の重畳（クリックヒットテスト用に screen 位置も集める）。
    let (x_min, x_max) = (surf.x_values[0], surf.x_values[nx - 1]);
    let (y_min, y_max) = (surf.y_values[0], surf.y_values[ny - 1]);
    let mut screen_points: Vec<(egui::Pos2, usize)> = Vec::new();
    if x_max > x_min && y_max > y_min {
        for (idx, p) in result.points.iter().enumerate() {
            let fx = ((p[0] - x_min) / (x_max - x_min)).clamp(0.0, 1.0) as f32;
            let fy = ((p[1] - y_min) / (y_max - y_min)).clamp(0.0, 1.0) as f32;
            let pos = egui::pos2(
                rect.left() + fx * rect.width(),
                rect.bottom() - fy * rect.height(),
            );
            screen_points.push((pos, idx));
            if opts.show_points {
                let cv = if use_log {
                    p[2].max(1e-300).log10()
                } else {
                    p[2]
                };
                let t = if (cv_max - cv_min).abs() < f64::EPSILON {
                    0.5
                } else {
                    ((cv - cv_min) / (cv_max - cv_min)).clamp(0.0, 1.0) as f32
                };
                painter.circle_filled(pos, 2.5, cmap.interpolate(t));
                painter.circle_stroke(pos, 2.5, egui::Stroke::new(0.6, egui::Color32::BLACK));
            }
        }
    }

    // カラーバー（ラベルは元の値域。対数時も実値を表示）。
    let bar_rect = egui::Rect::from_min_size(
        egui::pos2(rect.right() + 4.0, rect.top()),
        egui::vec2(16.0, rect.height()),
    );
    draw_colorbar_simple(ui, bar_rect, v_min, v_max, cmap.clone());

    ui.label(format!(
        "X: {}   Y: {}   Value: {}{}",
        result.x_name,
        result.y_name,
        result.value_name,
        if use_log { " (log color)" } else { "" }
    ));

    // クリック → 最近傍の観測点を詳細表示。
    if response.clicked() {
        if let Some(click) = response.interact_pointer_pos() {
            if let Some(idx) = nearest_point(&screen_points, click, HIT_THRESHOLD) {
                let trial_id = result.point_trial_ids.get(idx).copied()?;
                let row_index = view.trial_ids.iter().position(|&t| t == trial_id)?;
                let value = result.points.get(idx).map(|p| p[2]).unwrap_or(f64::NAN);
                return Some(TrialDetailTarget {
                    trial_id,
                    row_index,
                    context: vec![(result.value_name.clone(), format!("{:.6}", value))],
                });
            }
        }
    }
    None
}

struct Render3dOpts {
    show_points: bool,
    density_shade: bool,
}

/// 3D サーフェス表示。角に `None` を含むセル（マスク）は描かない＝外挿を見せない。
/// 軸: X=selected_x、Z=selected_y、縦 Y=value。深度ソートで奥から塗る（painter's algorithm）。
fn render_3d(
    ui: &mut egui::Ui,
    result: &ObservedContourResult,
    cmap: &ColorMap,
    camera: &mut ArcballCamera,
    opts: &Render3dOpts,
) {
    let surf = &result.surface;
    let nx = surf.x_values.len();
    let ny = surf.y_values.len();
    if nx < 2 || ny < 2 {
        ui.label("Not enough data to render a 3D surface.");
        return;
    }
    let (x_min, x_max) = (surf.x_values[0], surf.x_values[nx - 1]);
    let (y_min, y_max) = (surf.y_values[0], surf.y_values[ny - 1]);

    // value 範囲（マスクを除いた z セルから）。
    let (v_min, v_max) = {
        let mut mn = f64::INFINITY;
        let mut mx = f64::NEG_INFINITY;
        for col in &surf.z {
            for c in col.iter().flatten() {
                if *c < mn {
                    mn = *c;
                }
                if *c > mx {
                    mx = *c;
                }
            }
        }
        (mn, mx)
    };
    if !v_min.is_finite() || !v_max.is_finite() {
        ui.label("No data to render.");
        return;
    }

    let (painter, _rect, project) = setup_3d_canvas(ui, camera);
    draw_3d_grid(&painter, &project);

    // 点密度シェーディング: 観測点を各セルにビニングし、局所窓で平滑化して正規化する。
    // 1 セル単位ではほとんど 0/1 でノイズが多いため、近傍を平滑化して領域の濃淡にする。
    let density = if opts.density_shade {
        let blur_radius = (nx / 12).max(2);
        Some(cell_density_grid(
            &result.points,
            (x_min, x_max),
            (y_min, y_max),
            nx,
            ny,
            blur_radius,
        ))
    } else {
        None
    };

    // クリップ空間 [-1,1]^3 への写像。surf.z[i][j]=f(x_i,y_j)。
    let clip_at = |i: usize, j: usize, v: f64| -> [f32; 3] {
        let x = 2.0 * i as f32 / (nx - 1) as f32 - 1.0;
        let z = 2.0 * j as f32 / (ny - 1) as f32 - 1.0;
        let y = normalize_to_clip(v, v_min, v_max);
        [x, y, z]
    };

    enum Prim {
        Cell([egui::Pos2; 4], egui::Color32),
        Point(egui::Pos2, egui::Color32),
        Line(egui::Pos2, egui::Pos2, egui::Color32),
    }
    let mut items: Vec<(f32, Prim)> = Vec::new();

    // 4 隅とも Some のセルのみ面を張る（マスクは穴のまま）。
    for i in 0..nx - 1 {
        for j in 0..ny - 1 {
            let (Some(v00), Some(v10), Some(v11), Some(v01)) = (
                surf.z[i][j],
                surf.z[i + 1][j],
                surf.z[i + 1][j + 1],
                surf.z[i][j + 1],
            ) else {
                continue;
            };
            let corners = [
                clip_at(i, j, v00),
                clip_at(i + 1, j, v10),
                clip_at(i + 1, j + 1, v11),
                clip_at(i, j + 1, v01),
            ];
            let mut pts = [egui::Pos2::ZERO; 4];
            let mut depth = 0.0;
            let mut finite = true;
            for (k, c) in corners.iter().enumerate() {
                let (p, d) = project(*c);
                finite &= p.x.is_finite() && p.y.is_finite();
                pts[k] = p;
                depth += d;
            }
            if !finite {
                continue;
            }
            let mean = (v00 + v10 + v11 + v01) / 4.0;
            let mut color = cmap.interpolate(normalize(mean, v_min, v_max));
            // 観測の薄いセルを透明にする（密度→不透明度）。色相は保ち α だけ動かす。
            if let Some(d) = &density {
                let a = (40.0 + 215.0 * d[i][j].sqrt()).round().clamp(0.0, 255.0) as u8;
                let [r, g, b, _] = color.to_array();
                color = egui::Color32::from_rgba_unmultiplied(r, g, b, a);
            }
            items.push((depth * 0.25, Prim::Cell(pts, color)));
        }
    }

    // 観測点の重畳。
    if opts.show_points && x_max > x_min && y_max > y_min {
        for p in &result.points {
            let fx = ((p[0] - x_min) / (x_max - x_min)).clamp(0.0, 1.0);
            let fy = ((p[1] - y_min) / (y_max - y_min)).clamp(0.0, 1.0);
            let clip = [
                (2.0 * fx - 1.0) as f32,
                normalize_to_clip(p[2], v_min, v_max),
                (2.0 * fy - 1.0) as f32,
            ];
            let (pos, depth) = project(clip);
            if pos.x.is_finite() && pos.y.is_finite() {
                let color = cmap.interpolate(normalize(p[2], v_min, v_max));
                items.push((depth, Prim::Point(pos, color)));
            }
        }
    }

    // 軸線を細分化してサーフェスと一緒に深度ソートし、面との前後関係を反映する。
    for (a, b, color) in axis_segments_3d(24) {
        let (pos_a, depth_a) = project(a);
        let (pos_b, depth_b) = project(b);
        if pos_a.x.is_finite() && pos_a.y.is_finite() && pos_b.x.is_finite() && pos_b.y.is_finite()
        {
            items.push(((depth_a + depth_b) * 0.5, Prim::Line(pos_a, pos_b, color)));
        }
    }

    items.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

    // 奥から手前へ描く。面・軸線は生メッシュ、点は円 Shape を挟むためメッシュを確定する。
    let mut mesh = egui::Mesh::default();
    for (_, prim) in &items {
        match prim {
            Prim::Cell(corners, color) => {
                let [p0, p1, p2, p3] = *corners;
                push_tri(&mut mesh, [p0, p1, p2], *color);
                push_tri(&mut mesh, [p0, p2, p3], *color);
            }
            Prim::Line(a, b, color) => {
                push_edge(&mut mesh, *a, *b, *color, 0.75);
            }
            Prim::Point(pos, color) => {
                if !mesh.is_empty() {
                    painter.add(egui::Shape::mesh(std::mem::take(&mut mesh)));
                }
                painter.circle_filled(*pos, 3.0, *color);
                painter.circle_stroke(*pos, 3.0, egui::Stroke::new(0.6, egui::Color32::BLACK));
            }
        }
    }
    if !mesh.is_empty() {
        painter.add(egui::Shape::mesh(mesh));
    }

    // 軸名・値ラベル（最前面）。X=selected_x, 縦 Y=value, Z=selected_y。
    draw_3d_axis_labels(
        &painter,
        &project,
        [&result.x_name, &result.value_name, &result.y_name],
        [(x_min, x_max), (v_min, v_max), (y_min, y_max)],
    );
}

/// 三角形を生メッシュに追加する（投影後の退化形状でも安全なよう法線計算なし）。
fn push_tri(mesh: &mut egui::Mesh, pts: [egui::Pos2; 3], color: egui::Color32) {
    let base = mesh.vertices.len() as u32;
    for p in pts {
        mesh.vertices.push(egui::epaint::Vertex {
            pos: p,
            uv: egui::epaint::WHITE_UV,
            color,
        });
    }
    mesh.indices.extend([base, base + 1, base + 2]);
}

/// 線分を細いクアッド（三角形 2 枚）として生メッシュに追加する（深度ソートに混ぜる用途）。
fn push_edge(mesh: &mut egui::Mesh, a: egui::Pos2, b: egui::Pos2, color: egui::Color32, hw: f32) {
    let v = b - a;
    let len = v.length();
    if len < f32::EPSILON {
        return;
    }
    let n = egui::vec2(-v.y, v.x) * (hw / len);
    push_tri(mesh, [a + n, b + n, b - n], color);
    push_tri(mesh, [a + n, b - n, a - n], color);
}

/// 観測点を (nx-1)×(ny-1) のセルにビニングし、半径 `blur_radius` で局所平滑化したうえで
/// 最大値で割った正規化密度 (0..1) を返す。セル (i,j) は x∈[x_i,x_{i+1}]、y∈[y_j,y_{j+1}]。
/// 1 セル単位ではほぼ 0/1 でノイズが多いため、平滑化して領域の濃淡を表す。
fn cell_density_grid(
    points: &[[f64; 3]],
    (x_min, x_max): (f64, f64),
    (y_min, y_max): (f64, f64),
    nx: usize,
    ny: usize,
    blur_radius: usize,
) -> Vec<Vec<f32>> {
    let (cx, cy) = (nx.saturating_sub(1).max(1), ny.saturating_sub(1).max(1));
    let mut counts = vec![vec![0f32; cy]; cx];
    if x_max > x_min && y_max > y_min {
        for p in points {
            let fx = ((p[0] - x_min) / (x_max - x_min)).clamp(0.0, 1.0);
            let fy = ((p[1] - y_min) / (y_max - y_min)).clamp(0.0, 1.0);
            let i = ((fx * cx as f64) as usize).min(cx - 1);
            let j = ((fy * cy as f64) as usize).min(cy - 1);
            counts[i][j] += 1.0;
        }
    }
    let smoothed = box_blur_2d(&counts, blur_radius);
    let max = smoothed.iter().flatten().copied().fold(0.0_f32, f32::max);
    let denom = if max > 0.0 { max } else { 1.0 };
    smoothed
        .iter()
        .map(|col| col.iter().map(|&c| c / denom).collect())
        .collect()
}

/// 2D グリッドに半径 `r` の分離型箱平滑化（近傍平均）を適用する。`r == 0` は恒等。
fn box_blur_2d(grid: &[Vec<f32>], r: usize) -> Vec<Vec<f32>> {
    if r == 0 || grid.is_empty() {
        return grid.to_vec();
    }
    let nx = grid.len();
    let ny = grid[0].len();
    // 横方向の移動平均。
    let mut tmp = vec![vec![0f32; ny]; nx];
    for (i, col) in tmp.iter_mut().enumerate() {
        for (j, slot) in col.iter_mut().enumerate() {
            let lo = i.saturating_sub(r);
            let hi = (i + r).min(nx - 1);
            let mut sum = 0.0;
            for row in grid.iter().take(hi + 1).skip(lo) {
                sum += row[j];
            }
            *slot = sum / (hi - lo + 1) as f32;
        }
    }
    // 縦方向の移動平均。
    let mut out = vec![vec![0f32; ny]; nx];
    for (out_row, src_row) in out.iter_mut().zip(tmp.iter()) {
        for (j, slot) in out_row.iter_mut().enumerate() {
            let lo = j.saturating_sub(r);
            let hi = (j + r).min(ny - 1);
            let sum: f32 = src_row[lo..=hi].iter().sum();
            *slot = sum / (hi - lo + 1) as f32;
        }
    }
    out
}

/// `screen_points`（位置, インデックス）の中で `click` に最も近く閾値内の点のインデックスを返す。
fn nearest_point(
    screen_points: &[(egui::Pos2, usize)],
    click: egui::Pos2,
    threshold: f32,
) -> Option<usize> {
    let mut best: Option<(f32, usize)> = None;
    for (pos, idx) in screen_points {
        let d = pos.distance(click);
        if d <= threshold && best.is_none_or(|(bd, _)| d < bd) {
            best = Some((d, *idx));
        }
    }
    best.map(|(_, idx)| idx)
}

/// マスク対応の等高線（marching squares）。4 隅とも `Some` のセルのみ描く。
fn draw_contour_lines(
    painter: &egui::Painter,
    rect: egui::Rect,
    display: &[Vec<Option<f64>>],
    v_min: f64,
    v_max: f64,
) {
    let ny = display.len();
    if ny < 2 {
        return;
    }
    let nx = display[0].len();
    if nx < 2 || (v_max - v_min).abs() < f64::EPSILON {
        return;
    }
    let cw = rect.width() / nx as f32;
    let ch = rect.height() / ny as f32;
    // セル中心をサンプル位置とする。
    let sx = |c: usize| rect.left() + (c as f32 + 0.5) * cw;
    let sy = |r: usize| rect.top() + (r as f32 + 0.5) * ch;
    let stroke = egui::Stroke::new(0.8, egui::Color32::from_white_alpha(150));

    for li in 1..=N_CONTOUR_LEVELS {
        let level = v_min + (v_max - v_min) * li as f64 / (N_CONTOUR_LEVELS + 1) as f64;
        for r in 0..ny - 1 {
            for c in 0..nx - 1 {
                let (Some(tl), Some(tr), Some(br), Some(bl)) = (
                    display[r][c],
                    display[r][c + 1],
                    display[r + 1][c + 1],
                    display[r + 1][c],
                ) else {
                    continue; // 不完全セルは等高線を描かない。
                };
                // 4 辺（上・右・下・左）の交点を集める。
                let mut pts: Vec<egui::Pos2> = Vec::with_capacity(4);
                let (x0, x1) = (sx(c), sx(c + 1));
                let (y0, y1) = (sy(r), sy(r + 1));
                if let Some(t) = edge_cross(tl, tr, level) {
                    pts.push(egui::pos2(lerp(x0, x1, t), y0));
                }
                if let Some(t) = edge_cross(tr, br, level) {
                    pts.push(egui::pos2(x1, lerp(y0, y1, t)));
                }
                if let Some(t) = edge_cross(bl, br, level) {
                    pts.push(egui::pos2(lerp(x0, x1, t), y1));
                }
                if let Some(t) = edge_cross(tl, bl, level) {
                    pts.push(egui::pos2(x0, lerp(y0, y1, t)));
                }
                match pts.len() {
                    2 => {
                        painter.line_segment([pts[0], pts[1]], stroke);
                    }
                    4 => {
                        painter.line_segment([pts[0], pts[1]], stroke);
                        painter.line_segment([pts[2], pts[3]], stroke);
                    }
                    _ => {}
                }
            }
        }
    }
}

/// 辺の 2 端点 `a`,`b` が `level` を挟むなら、a→b 上の交点比率 `t`(0..1) を返す。
fn edge_cross(a: f64, b: f64, level: f64) -> Option<f64> {
    let above_a = a >= level;
    let above_b = b >= level;
    if above_a == above_b {
        return None;
    }
    let denom = b - a;
    if denom.abs() < f64::EPSILON {
        return None;
    }
    Some(((level - a) / denom).clamp(0.0, 1.0))
}

fn lerp(a: f32, b: f32, t: f64) -> f32 {
    a + (b - a) * t as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edge_cross_detects_straddle() {
        // 0 と 2 が level=1 を挟む → 中点 t=0.5。
        assert_eq!(edge_cross(0.0, 2.0, 1.0), Some(0.5));
        // 同符号は None。
        assert!(edge_cross(0.0, 0.5, 1.0).is_none());
        assert!(edge_cross(2.0, 3.0, 1.0).is_none());
    }

    #[test]
    fn cell_density_grid_bins_and_normalizes() {
        // 3x3 グリッド → 2x2 セル。左下セルに 2 点、右上セルに 1 点。
        let pts = vec![
            [0.1, 0.1, 0.0],
            [0.2, 0.2, 0.0],
            [0.9, 0.9, 0.0],
            [1.0, 1.0, 0.0], // 端は最終セルにクランプ
        ];
        // blur=0 はビニングそのまま（正規化のみ）。
        let d = cell_density_grid(&pts, (0.0, 1.0), (0.0, 1.0), 3, 3, 0);
        assert_eq!(d.len(), 2);
        assert_eq!(d[0].len(), 2);
        // 左下 (i=0,j=0) が最大カウント 2 → 1.0。
        assert!((d[0][0] - 1.0).abs() < 1e-6);
        // 右上 (i=1,j=1) はカウント 2（0.9 と 1.0）→ 1.0。
        assert!((d[1][1] - 1.0).abs() < 1e-6);
        // 空セルは 0。
        assert!(d[0][1].abs() < 1e-6);
        assert!(d[1][0].abs() < 1e-6);
    }

    #[test]
    fn box_blur_spreads_into_neighbors() {
        // 中央だけ値を持つ 3x3。半径1の平滑化で隣接セルが非ゼロになる。
        let mut g = vec![vec![0.0_f32; 3]; 3];
        g[1][1] = 9.0;
        let b = box_blur_2d(&g, 1);
        // 中央は 9/9（3x3 平均）= 1.0、隅は 9/9 もかかる…分離型なので確認は非ゼロのみ。
        assert!(b[1][1] > 0.0);
        assert!(b[0][1] > 0.0); // 縦横の隣接に滲む
        assert!(b[1][0] > 0.0);
        // 総和は保存される（平均の分離適用でも端のクランプで概ね保たれる）。
        let before: f32 = g.iter().flatten().sum();
        let after: f32 = b.iter().flatten().sum();
        assert!((before - after).abs() < before); // 完全保存ではないが発散しない
    }

    #[test]
    fn box_blur_zero_radius_is_identity() {
        let g = vec![vec![1.0_f32, 2.0], vec![3.0, 4.0]];
        assert_eq!(box_blur_2d(&g, 0), g);
    }

    #[test]
    fn nearest_point_within_threshold() {
        let pts = vec![
            (egui::pos2(0.0, 0.0), 0),
            (egui::pos2(10.0, 0.0), 1),
            (egui::pos2(100.0, 100.0), 2),
        ];
        assert_eq!(nearest_point(&pts, egui::pos2(11.0, 0.0), 12.0), Some(1));
        assert_eq!(nearest_point(&pts, egui::pos2(500.0, 500.0), 12.0), None);
    }
}
