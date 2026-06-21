//! トライアル詳細モーダル内に描画するレーダーチャート。
//!
//! 軸（頂点）は目的関数 → 変数の順に並べる。各軸の半径スケールは
//! パレートフロント（`pareto_rank == 0`）個体の値域に合わせ、外周（radius = 1.0）が
//! パレートフロント最大値（= 包絡上限）に対応する。比較のため、パレートフロントの
//! 各個体を薄い線で重ね描きし、選択トライアルをその上に濃い赤で重ねる。
//!
//! `egui_plot` には極座標チャートが無いため、`egui::Painter` で自前描画する。
//! 軸スケール計算（[`axis_scale`] / [`value_fraction`]）と軸構築（[`build`]）は
//! 純粋関数として切り出し、描画ロジックと独立にテストする。

use std::f32::consts::PI;

use egui::Color32;

use crate::state::types::StudyView;
use crate::theme::{ACCENT_BLUE, ERROR_COLOR, TEXT_SECONDARY};

/// パレートフロント各個体の線色（アクセントブルー #3B82F6 を alpha≈48 で薄く）。
/// 重なるほど色が濃くなり、分布の密度が見える。`from_rgba_premultiplied` は const
/// のため、(59,130,246) を alpha 48 で事前乗算した値を直接指定する。
const FRONT_LINE: Color32 = Color32::from_rgba_premultiplied(11, 24, 46, 48);
/// 選択トライアル多角形の色（赤系で強調）。
const SELECTED: Color32 = ERROR_COLOR;
/// 選択トライアル多角形の塗り（半透明）。
const SELECTED_FILL: Color32 = Color32::from_rgba_premultiplied(120, 34, 27, 50);
/// グリッド（同心多角形・スポーク）の色。
const GRID: Color32 = Color32::from_gray(214);

/// レーダー 1 軸ぶんのメタ情報。
#[derive(Debug, Clone, PartialEq)]
pub struct RadarAxis {
    /// 軸名（目的関数名 または 変数名）。
    pub name: String,
    /// 目的関数なら true（変数なら false）。ラベル色分けに使う。
    pub is_objective: bool,
    /// 選択トライアルの値（欠損・非有限のとき None）。
    pub selected: Option<f64>,
    /// パレートフロント個体での最小値（半径スケールの下限算出に使う）。
    pub front_min: f64,
    /// パレートフロント個体での最大値（= 軸の包絡上限）。
    pub front_max: f64,
}

/// レーダーチャートの描画データ。
#[derive(Debug, Clone, PartialEq)]
pub struct RadarData {
    /// 軸メタ情報（目的 → 変数の順）。
    pub axes: Vec<RadarAxis>,
    /// パレートフロント各個体の値。外側 = 個体、内側 = `axes` と同じ並びの軸値。
    /// 欠損・非有限は None。
    pub front: Vec<Vec<Option<f64>>>,
}

/// `StudyView` から目的関数 → 変数の順でレーダー描画データを構築する。
///
/// パレートフロント（`pareto_rank == 0`）に有限値が無い軸はスキップする。
/// `front` には各パレートフロント個体の（スキップ後の）軸値を整列して格納する。
pub fn build(
    view: &StudyView,
    obj_names: &[String],
    param_names: &[String],
    selected_row: usize,
) -> RadarData {
    let front_rows: Vec<usize> = (0..view.row_count())
        .filter(|&i| view.pareto_rank.get(i).copied() == Some(0))
        .collect();

    // 採用する軸の (列スライス, メタ) を順に収集する。
    let mut axes: Vec<RadarAxis> = Vec::with_capacity(obj_names.len() + param_names.len());
    let mut cols: Vec<&[f64]> = Vec::with_capacity(axes.capacity());
    for (names, is_objective) in [(obj_names, true), (param_names, false)] {
        for name in names {
            let Some(col) = view.numeric_column(name) else {
                continue;
            };
            let mut lo = f64::INFINITY;
            let mut hi = f64::NEG_INFINITY;
            for &r in &front_rows {
                if let Some(&v) = col.get(r) {
                    if v.is_finite() {
                        lo = lo.min(v);
                        hi = hi.max(v);
                    }
                }
            }
            if !(lo.is_finite() && hi.is_finite()) {
                continue;
            }
            let selected = col.get(selected_row).copied().filter(|v| v.is_finite());
            axes.push(RadarAxis {
                name: name.clone(),
                is_objective,
                selected,
                front_min: lo,
                front_max: hi,
            });
            cols.push(col);
        }
    }

    // 各フロント個体の値を採用軸ぶんだけ整列して取り出す。
    let front: Vec<Vec<Option<f64>>> = front_rows
        .iter()
        .map(|&r| {
            cols.iter()
                .map(|col| col.get(r).copied().filter(|v| v.is_finite()))
                .collect()
        })
        .collect();

    RadarData { axes, front }
}

/// 軸の半径スケール `(lo, hi)` を返す。
///
/// `hi` はパレートフロント最大値（外周＝包絡上限）。`lo` はフロント最小値より下に
/// マージンを取り、フロント個体が中心から離れて見えるようにする。フロントが 1 点
/// （`front_min == front_max`）の場合は、その値が半径中央に来るよう対称に広げる。
pub fn axis_scale(front_min: f64, front_max: f64) -> (f64, f64) {
    let span = front_max - front_min;
    if span.abs() <= f64::EPSILON {
        let pad = front_max.abs().max(1.0);
        (front_max - pad, front_max + pad)
    } else {
        (front_min - span * 0.2, front_max)
    }
}

/// 値を半径割合へ写像する。範囲外はわずかな超過を許し、描画側でクランプする。
pub fn value_fraction(value: f64, lo: f64, hi: f64) -> f32 {
    let span = hi - lo;
    if span.abs() <= f64::EPSILON {
        return 0.5;
    }
    ((value - lo) / span) as f32
}

/// レーダーチャートを描画する。軸が 3 未満ならレーダーにならないため注記のみ表示。
pub fn show(ui: &mut egui::Ui, data: &RadarData) {
    let axes = &data.axes;
    if axes.len() < 3 {
        ui.label(
            egui::RichText::new("Radar chart needs at least 3 axes (objectives + variables).")
                .weak(),
        );
        return;
    }

    let n = axes.len();
    let side = ui.available_width().clamp(240.0, 460.0);
    let (rect, _resp) = ui.allocate_exact_size(egui::vec2(side, side), egui::Sense::hover());
    let painter = ui.painter_at(rect);
    let center = rect.center();
    // ラベルぶんの余白を引いた半径。
    let radius = side * 0.5 - 64.0;

    let scales: Vec<(f64, f64)> = axes
        .iter()
        .map(|a| axis_scale(a.front_min, a.front_max))
        .collect();

    // 頂点 i の角度（上方向起点・時計回り）。
    let angle = |i: usize| -> f32 { -PI / 2.0 + (i as f32) * 2.0 * PI / (n as f32) };
    // 頂点 i・半径割合 frac のスクリーン座標（過超過は 1.12 でクランプ）。
    let point_at = |i: usize, frac: f32| -> egui::Pos2 {
        let a = angle(i);
        let r = frac.clamp(0.0, 1.12) * radius;
        center + egui::vec2(a.cos() * r, a.sin() * r)
    };
    // 軸値の系列（欠損は None）をスクリーン座標へ写像する。
    let to_points = |values: &[Option<f64>]| -> Vec<Option<egui::Pos2>> {
        (0..n)
            .map(|i| {
                values.get(i).copied().flatten().map(|v| {
                    let (lo, hi) = scales[i];
                    point_at(i, value_fraction(v, lo, hi))
                })
            })
            .collect()
    };

    // ── グリッド（同心多角形 + スポーク）──────────────────────────
    for ring in [0.25_f32, 0.5, 0.75, 1.0] {
        let pts: Vec<egui::Pos2> = (0..n).map(|i| point_at(i, ring)).collect();
        painter.add(egui::Shape::closed_line(pts, egui::Stroke::new(1.0, GRID)));
    }
    for i in 0..n {
        painter.line_segment([center, point_at(i, 1.0)], egui::Stroke::new(1.0, GRID));
    }

    // ── パレートフロント各個体（薄い線で重ね描き）────────────────
    let front_stroke = egui::Stroke::new(1.0, FRONT_LINE);
    for individual in &data.front {
        let pts = to_points(individual);
        draw_ring_polyline(&painter, &pts, front_stroke);
    }

    // ── 選択トライアル ──────────────────────────────────────────
    let sel_values: Vec<Option<f64>> = axes.iter().map(|a| a.selected).collect();
    let sel_pts = to_points(&sel_values);

    if sel_pts.iter().all(|p| p.is_some()) {
        // 全軸そろっていれば中心からの扇状メッシュで塗る（中心に対し星形なので妥当）。
        let pts: Vec<egui::Pos2> = sel_pts.iter().map(|p| p.unwrap()).collect();
        let mut fill = egui::Mesh::default();
        fill.colored_vertex(center, SELECTED_FILL);
        for &p in &pts {
            fill.colored_vertex(p, SELECTED_FILL);
        }
        for i in 0..n {
            let a = 1 + i as u32;
            let b = 1 + ((i + 1) % n) as u32;
            fill.add_triangle(0, a, b);
        }
        painter.add(egui::Shape::mesh(fill));
        painter.add(egui::Shape::closed_line(
            pts,
            egui::Stroke::new(2.0, SELECTED),
        ));
    } else {
        // 欠損軸があれば隣接する有効頂点どうしだけ線で結ぶ。
        draw_ring_polyline(&painter, &sel_pts, egui::Stroke::new(2.0, SELECTED));
    }
    for p in sel_pts.iter().flatten() {
        painter.circle_filled(*p, 3.0, SELECTED);
    }

    // ── 軸ラベル ────────────────────────────────────────────────
    for (i, axis) in axes.iter().enumerate() {
        let a = angle(i);
        let lp = center + egui::vec2(a.cos() * (radius + 12.0), a.sin() * (radius + 12.0));
        let align = if a.cos().abs() < 0.3 {
            egui::Align2::CENTER_CENTER
        } else if a.cos() > 0.0 {
            egui::Align2::LEFT_CENTER
        } else {
            egui::Align2::RIGHT_CENTER
        };
        let color = if axis.is_objective {
            ACCENT_BLUE
        } else {
            TEXT_SECONDARY
        };
        painter.text(
            lp,
            align,
            &axis.name,
            egui::FontId::proportional(11.0),
            color,
        );
    }

    // ── 凡例 ────────────────────────────────────────────────────
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        swatch(ui, ACCENT_BLUE);
        ui.label(
            egui::RichText::new(format!("Pareto front individuals ({})", data.front.len())).small(),
        );
        ui.add_space(12.0);
        swatch(ui, SELECTED);
        ui.label(egui::RichText::new("This trial").small());
    });
    ui.label(
        egui::RichText::new(
            "Outer ring = Pareto front max (envelope). Objective axes in blue, variables in gray.",
        )
        .small()
        .weak(),
    );
}

/// 軸順の点列（欠損は None）を閉じた折れ線として描く。
/// 隣り合う有効頂点どうしのみ線分で結び、欠損があればその区間を飛ばす。
fn draw_ring_polyline(painter: &egui::Painter, pts: &[Option<egui::Pos2>], stroke: egui::Stroke) {
    let n = pts.len();
    for i in 0..n {
        let j = (i + 1) % n;
        if let (Some(a), Some(b)) = (pts[i], pts[j]) {
            painter.line_segment([a, b], stroke);
        }
    }
}

/// 凡例用の小さな色見本を描く。
fn swatch(ui: &mut egui::Ui, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
    ui.painter().rect_filled(rect, 2.0, color);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn axis_scale_outer_maps_to_front_max() {
        let (lo, hi) = axis_scale(2.0, 10.0);
        // 外周（hi）はフロント最大に一致。
        assert!((hi - 10.0).abs() < 1e-9);
        // 最小は下方向にマージン（span=8 の 20% 下）。
        assert!((lo - (2.0 - 1.6)).abs() < 1e-9);
        // フロント最大は割合 1.0、最小は中心から離れた正の割合。
        assert!((value_fraction(10.0, lo, hi) - 1.0).abs() < 1e-6);
        let f_min = value_fraction(2.0, lo, hi);
        assert!(f_min > 0.0 && f_min < 1.0);
    }

    #[test]
    fn axis_scale_handles_degenerate_front() {
        // フロントが 1 点なら値は半径中央。
        let (lo, hi) = axis_scale(5.0, 5.0);
        assert!((value_fraction(5.0, lo, hi) - 0.5).abs() < 1e-6);
        assert!(lo < 5.0 && hi > 5.0);
    }

    #[test]
    fn value_fraction_zero_span_is_center() {
        assert!((value_fraction(3.0, 1.0, 1.0) - 0.5).abs() < 1e-6);
    }

    #[test]
    fn build_objectives_first_then_params_and_collects_front() {
        use crate::state::types::TrialState;
        use std::collections::HashMap;
        use std::sync::Arc;
        use tunny_core::dataframe::{DataFrame, TrialRow as CoreRow};

        // 3 トライアル。trial0,1 がフロント（rank 0）、trial2 は rank 1。
        let core_rows: Vec<CoreRow> = (0..3)
            .map(|i| CoreRow {
                trial_id: i as u32,
                trial_number: i as u32,
                param_display: HashMap::from([("x".to_string(), i as f64)]),
                param_category_label: HashMap::new(),
                objective_values: vec![i as f64 * 2.0, 10.0 - i as f64],
                user_attrs_numeric: HashMap::new(),
                user_attrs_string: HashMap::new(),
                constraint_values: vec![],
            })
            .collect();
        let obj_names = vec!["o0".to_string(), "o1".to_string()];
        let param_names = vec!["x".to_string()];
        let df = DataFrame::from_trials(&core_rows, &param_names, &obj_names, &[], &[], 0);
        let mut view = StudyView::new(Arc::new(df), vec![0, 0, 1]);
        view.state = vec![TrialState::Complete; 3];

        let data = build(&view, &obj_names, &param_names, 0);
        // 目的 2 + 変数 1 = 3 軸、順序は目的→変数。
        assert_eq!(data.axes.len(), 3);
        assert_eq!(data.axes[0].name, "o0");
        assert!(data.axes[0].is_objective);
        assert_eq!(data.axes[2].name, "x");
        assert!(!data.axes[2].is_objective);

        // フロント（trial0,1）のみで min/max を取る: o0 = {0,2} → [0,2]。
        assert!((data.axes[0].front_min - 0.0).abs() < 1e-9);
        assert!((data.axes[0].front_max - 2.0).abs() < 1e-9);
        // 選択 = row 0 の o0 = 0.0。
        assert_eq!(data.axes[0].selected, Some(0.0));

        // フロント個体は 2 行、各 3 軸ぶん。trial1 の o0=2, o1=9, x=1。
        assert_eq!(data.front.len(), 2);
        assert_eq!(data.front[0].len(), 3);
        assert_eq!(data.front[1], vec![Some(2.0), Some(9.0), Some(1.0)]);
    }

    #[test]
    fn build_skips_axis_without_front_values() {
        use std::collections::HashMap;
        use std::sync::Arc;
        use tunny_core::dataframe::{DataFrame, TrialRow as CoreRow};

        let core_rows: Vec<CoreRow> = (0..2)
            .map(|i| CoreRow {
                trial_id: i,
                trial_number: i,
                param_display: HashMap::new(),
                param_category_label: HashMap::new(),
                objective_values: vec![i as f64],
                user_attrs_numeric: HashMap::new(),
                user_attrs_string: HashMap::new(),
                constraint_values: vec![],
            })
            .collect();
        let obj_names = vec!["o0".to_string()];
        let df = DataFrame::from_trials(&core_rows, &[], &obj_names, &[], &[], 0);
        let view = StudyView::new(Arc::new(df), vec![0, 0]);
        // 存在しない列名を要求してスキップを確認する。
        let missing = vec!["nope".to_string()];
        let data = build(&view, &missing, &[], 0);
        assert!(data.axes.is_empty());
        // 軸が無ければフロント各行も空。
        assert!(data.front.iter().all(|r| r.is_empty()));
    }
}
