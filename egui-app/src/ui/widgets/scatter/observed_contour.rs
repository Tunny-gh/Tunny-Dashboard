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
use crate::ui::widgets::trial_detail_modal::{
    show_hover_tooltip, TrialDetailTarget, HIT_THRESHOLD,
};

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
                modal_open: state.detail_modal.is_open(),
            };
            render_3d(ui, result, &cmap, &mut state.camera, &opts3d, view)
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
    // 右にラベル付きカラーバー（バー＋数値目盛＋縦書きの値名）、下にサブタイトル1行ぶんを確保する。
    const COLORBAR_RESERVE: f32 = 96.0;
    const CAPTION_RESERVE: f32 = 28.0;
    let plot_size = egui::vec2(
        (available.width() - COLORBAR_RESERVE).max(120.0),
        (available.height() - CAPTION_RESERVE).clamp(120.0, 360.0),
    );
    let (rect, response) = ui.allocate_exact_size(plot_size, egui::Sense::click());
    let painter = ui.painter_at(rect);

    painter.rect_stroke(
        rect,
        0.0,
        egui::Stroke::new(1.0, egui::Color32::from_gray(80)),
        egui::StrokeKind::Inside,
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

    // ラベル付きカラーバー（バー＋数値目盛＋縦書きの値名）。数値は元の値域（対数色でも実値）。
    let bar_rect = egui::Rect::from_min_size(
        egui::pos2(rect.right() + 6.0, rect.top()),
        egui::vec2(14.0, rect.height()),
    );
    let title = if use_log {
        format!("{} (log)", result.value_name)
    } else {
        result.value_name.clone()
    };
    draw_colorbar_simple(ui, bar_rect, v_min, v_max, cmap.clone(), Some(&title));

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
    /// 詳細モーダル表示中はホバーツールチップを抑止する。
    modal_open: bool,
}

/// 3D サーフェス表示。角に `None` を含むセル（マスク）は描かない＝外挿を見せない。
/// 軸: X=selected_x、Z=selected_y、縦 Y=value。深度ソートで奥から塗る（painter's algorithm）。
/// 観測点（Show points 時のみ）のホバーでツールチップを、クリックで詳細対象を返す。
fn render_3d(
    ui: &mut egui::Ui,
    result: &ObservedContourResult,
    cmap: &ColorMap,
    camera: &mut ArcballCamera,
    opts: &Render3dOpts,
    view: &StudyView,
) -> Option<TrialDetailTarget> {
    let surf = &result.surface;
    let nx = surf.x_values.len();
    let ny = surf.y_values.len();
    if nx < 2 || ny < 2 {
        ui.label("Not enough data to render a 3D surface.");
        return None;
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
        return None;
    }

    let (painter, _rect, project, click_pos, hover_pos) = setup_3d_canvas(ui, camera);
    draw_3d_grid(&painter, &project);

    // 点密度シェーディング: 観測点を各セルにビニングし、局所窓で平滑化して正規化する。
    // 1 セル単位ではほとんど 0/1 でノイズが多いため、近傍を平滑化して領域の濃淡にする。
    let density = if opts.density_shade {
        let blur_radius = (nx / 12).max(2);
        Some(tunny_core::contour::cell_density_grid(
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

    // 観測点の重畳（ホバー・クリックのヒットテスト用に screen 位置も集める）。
    let mut screen_points: Vec<(egui::Pos2, usize)> = Vec::new();
    if opts.show_points && x_max > x_min && y_max > y_min {
        for (idx, p) in result.points.iter().enumerate() {
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
                screen_points.push((pos, idx));
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

    // ヒットした観測点から詳細対象を組み立てる（ホバー・クリック共用）。
    let target_at = |idx: usize| -> Option<TrialDetailTarget> {
        let trial_id = result.point_trial_ids.get(idx).copied()?;
        let row_index = view.trial_ids.iter().position(|&t| t == trial_id)?;
        let value = result.points.get(idx).map(|p| p[2]).unwrap_or(f64::NAN);
        Some(TrialDetailTarget {
            trial_id,
            row_index,
            context: vec![(result.value_name.clone(), format!("{:.6}", value))],
        })
    };

    // ホバー → ツールチップ（2D 版・他の 3D 散布図と同じ操作感。モーダル表示中は抑止）。
    if !opts.modal_open {
        if let Some(hover) = hover_pos {
            if let Some(idx) = nearest_point(&screen_points, hover, HIT_THRESHOLD) {
                if let Some(target) = target_at(idx) {
                    let trial_number = view
                        .df
                        .get_trial_number(target.row_index)
                        .unwrap_or(target.row_index as u32);
                    let p = &result.points[idx];
                    let rows = vec![
                        (result.x_name.clone(), format!("{:.4}", p[0])),
                        (result.y_name.clone(), format!("{:.4}", p[1])),
                        (result.value_name.clone(), format!("{:.4}", p[2])),
                    ];
                    show_hover_tooltip(ui, "observed_contour3d_hover_tooltip", trial_number, &rows);
                }
            }
        }
    }

    // クリック → 最近傍の観測点を詳細表示。
    if let Some(click) = click_pos {
        if let Some(idx) = nearest_point(&screen_points, click, HIT_THRESHOLD) {
            return target_at(idx);
        }
    }
    None
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

/// マスク対応の等高線（marching squares）。セグメント抽出は tunny_core に委譲し、
/// ここではグリッドのサンプル index 空間 → スクリーン座標の写像と描画のみ行う。
fn draw_contour_lines(
    painter: &egui::Painter,
    rect: egui::Rect,
    display: &[Vec<Option<f64>>],
    v_min: f64,
    v_max: f64,
) {
    let ny = display.len();
    if ny == 0 {
        return;
    }
    let nx = display[0].len();
    if nx == 0 {
        return;
    }
    let cw = rect.width() / nx as f32;
    let ch = rect.height() / ny as f32;
    // セル中心をサンプル位置とする。
    let to_screen = |p: [f64; 2]| {
        egui::pos2(
            rect.left() + (p[0] as f32 + 0.5) * cw,
            rect.top() + (p[1] as f32 + 0.5) * ch,
        )
    };
    let stroke = egui::Stroke::new(0.8, egui::Color32::from_white_alpha(150));

    for (a, b) in
        tunny_core::contour::contour_line_segments(display, v_min, v_max, N_CONTOUR_LEVELS)
    {
        painter.line_segment([to_screen(a), to_screen(b)], stroke);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // edge_cross / cell_density_grid / box_blur_2d のテストは
    // rust_core/src/contour/mod.rs へ移設した（数値処理の移行に伴う）。

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
