use crate::state::messages::PdpResult2d;
use crate::state::types::StudyView;
use crate::theme::chart_colors::{COLOR_CONTOUR, COLOR_PDP_CI};
use crate::theme::colormap::ColorMap;
use crate::ui::widgets::common::heatmap::draw_colorbar_simple;
use crate::ui::widgets::common::range_math;
use crate::ui::widgets::pdp_chart::{classify_observed, ModelType, ObservedKind};
use crate::ui::widgets::scatter_3d::{
    axis_segments_3d, draw_3d_axis_labels, draw_3d_grid, normalize_to_clip, setup_3d_canvas,
    ArcballCamera,
};

/// 2D グリッド値（行 = param1、列 = param2）
pub(crate) type Grid = Vec<Vec<f64>>;
/// 95% CI バンドの (下限, 上限) グリッド
pub(crate) type BandGrids = (Grid, Grid);

/// Pending 2D PDP computation request, placed by show() and consumed by the chart cell body.
pub struct Pdp2dComputeRequest {
    pub param1: String,
    pub param2: String,
    pub objective: String,
    pub n_grid: usize,
    pub model_type: String,
    /// 実行可能解（is_feasible > 0.5）のみでモデルをフィットするか
    pub feasible_only: bool,
}

/// PDP 2D ウィジェット状態
pub struct PdpChart2DState {
    pub selected_param1: String,
    pub selected_param2: String,
    pub selected_objective: usize,
    pub selected_model: ModelType,
    pub result: Option<PdpResult2d>,
    pub computing: bool,
    pub pending_compute: Option<Pdp2dComputeRequest>,
    pub camera: ArcballCamera,
    /// ガウス過程系で不確実性（±1.96σ = 95% CI）を半透明バンドとして重ねるか
    pub show_uncertainty: bool,
    /// 観測データ（サンプリング点）をサーフェスに重ねて表示するか
    pub show_observed: bool,
    /// 実行可能解のみでモデルをフィットするか（制約付きスタディのみ UI 表示）
    pub feasible_only: bool,
}

impl Default for PdpChart2DState {
    fn default() -> Self {
        Self {
            selected_param1: String::new(),
            selected_param2: String::new(),
            selected_objective: 0,
            selected_model: ModelType::Ridge,
            result: None,
            computing: false,
            pending_compute: None,
            camera: ArcballCamera::isometric_default(),
            show_uncertainty: true,
            show_observed: false,
            feasible_only: false,
        }
    }
}

impl PdpChart2DState {
    /// グローバル widget の計算実行状態・結果を取り込む。
    /// 2D PDP 結果は widget 側（result）に保持されるため、キャンバスの各アイテム
    /// （独立した WidgetStates）にも反映する。パラメータ・目的関数・モデルの選択は維持する。
    pub fn adopt_compute_state(&mut self, src: &Self) {
        self.computing = src.computing;
        self.result = src.result.clone();
    }

    #[allow(clippy::too_many_arguments)]
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        param_names: &[String],
        obj_names: &[String],
        cmap: ColorMap,
        view: &StudyView,
        selected_indices: &[u32],
        pinned: &[u32],
    ) {
        // Row 1: Parameter 1 + Parameter 2
        ui.horizontal(|ui| {
            ui.label("Parameter 1:");
            egui::ComboBox::from_id_salt("pdp2d_p1")
                .selected_text(&self.selected_param1)
                .show_ui(ui, |ui| {
                    for name in param_names {
                        ui.selectable_value(&mut self.selected_param1, name.clone(), name);
                    }
                });
            ui.label("Parameter 2:");
            egui::ComboBox::from_id_salt("pdp2d_p2")
                .selected_text(&self.selected_param2)
                .show_ui(ui, |ui| {
                    for name in param_names {
                        ui.selectable_value(&mut self.selected_param2, name.clone(), name);
                    }
                });
        });

        // Row 2: Objective + Model selector
        ui.horizontal(|ui| {
            ui.label("Objective:");
            let obj_text = obj_names
                .get(self.selected_objective)
                .map(|s| s.as_str())
                .unwrap_or("—");
            egui::ComboBox::from_id_salt("pdp2d_obj")
                .selected_text(obj_text)
                .show_ui(ui, |ui| {
                    for (i, name) in obj_names.iter().enumerate() {
                        if ui
                            .selectable_label(self.selected_objective == i, name)
                            .clicked()
                        {
                            self.selected_objective = i;
                        }
                    }
                });

            ui.label("Model:");
            egui::ComboBox::from_id_salt("pdp2d_model")
                .selected_text(self.selected_model.label())
                .show_ui(ui, |ui| {
                    for model in ModelType::ALL {
                        let selected = self.selected_model == model;
                        if ui.selectable_label(selected, model.label()).clicked() {
                            self.selected_model = model;
                        }
                    }
                });

            // 観測データ表示トグル（1D PDP と同じ操作感）
            ui.separator();
            ui.toggle_value(&mut self.show_observed, "Show data");

            // 実行可能解フィルタ（制約付きスタディのみ）
            if view.feasibility().has_constraints() {
                ui.toggle_value(&mut self.feasible_only, "Feasible only")
                    .on_hover_text("Fit the model using feasible trials only");
            }
        });

        // 同一パラメータ警告
        if !self.selected_param1.is_empty() && self.selected_param1 == self.selected_param2 {
            ui.colored_label(COLOR_CONTOUR, "Warning: the same parameter is selected");
        }

        // Run button — only enabled when params are different and objectives exist
        let can_run = check_params_different(&self.selected_param1, &self.selected_param2)
            && !obj_names.is_empty()
            && !self.computing;
        if ui
            .add_enabled(can_run, egui::Button::new("Run 2D PDP"))
            .clicked()
        {
            if let Some(obj_name) = obj_names.get(self.selected_objective) {
                let n_grid = match self.selected_model {
                    ModelType::RandomForest => 30,
                    _ => 20,
                };
                self.pending_compute = Some(Pdp2dComputeRequest {
                    param1: self.selected_param1.clone(),
                    param2: self.selected_param2.clone(),
                    objective: obj_name.clone(),
                    n_grid,
                    model_type: self.selected_model.to_str().to_string(),
                    feasible_only: self.feasible_only,
                });
            }
        }

        if self.computing {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("Computing 2D PDP...");
            });
            return;
        }

        if self.result.is_none() {
            ui.label("No 2D PDP data");
            return;
        }

        // 不確実性バンド表示トグル（ガウス過程系のみ。result の不変借用前に self を可変借用する）
        let has_uncertainty = self
            .result
            .as_ref()
            .is_some_and(|r| r.uncertainties.is_some());
        if has_uncertainty {
            ui.checkbox(&mut self.show_uncertainty, "95% CI (±1.96σ)");
        }

        let camera = &mut self.camera;
        let result = self.result.as_ref().unwrap();
        let values: &[Vec<f64>] = &result.z_values;
        let value_label = result.objective_name.clone();

        if values.len() < 2 || values[0].len() < 2 {
            ui.label("Not enough grid data for 3D surface");
            return;
        }

        // 不確実性バンド: Mean ± 1.96σ の上下サーフェス（半透明で重ね描き）
        let bands: Option<BandGrids> = if self.show_uncertainty && has_uncertainty {
            result
                .uncertainties
                .as_ref()
                .map(|unc| band_grids(values, unc))
        } else {
            None
        };

        // 観測データ ([param1, param2, objective], 分類)
        let observed: Vec<([f64; 3], ObservedKind)> = if self.show_observed {
            extract_observed_3d(
                view,
                &result.param1_name,
                &result.param2_name,
                &result.objective_name,
                selected_indices,
                pinned,
            )
        } else {
            vec![]
        };

        // 色は Mean の値域で正規化する（カラーバーもこの値域）
        let (c_min, c_max) = value_range_of(values);
        // 縦軸のジオメトリ範囲はバンド・観測点も収まるよう拡張する
        let (mut v_min, mut v_max) = (c_min, c_max);
        if let Some((lower, upper)) = &bands {
            let (l_min, _) = value_range_of(lower);
            let (_, u_max) = value_range_of(upper);
            v_min = v_min.min(l_min);
            v_max = v_max.max(u_max);
        }
        for (p, _) in &observed {
            v_min = v_min.min(p[2]);
            v_max = v_max.max(p[2]);
        }
        let (x_min, x_max) = axis_range_of(&result.x_values);
        let (y_min, y_max) = axis_range_of(&result.y_values);

        // 観測点をクリップ空間へ（X = param1, Y(縦) = 目的関数値, Z = param2）
        let observed_clip: Vec<([f32; 3], egui::Color32)> = observed
            .iter()
            .map(|&([p1, p2, ov], kind)| {
                (
                    [
                        normalize_to_clip(p1, x_min, x_max),
                        normalize_to_clip(ov, v_min, v_max),
                        normalize_to_clip(p2, y_min, y_max),
                    ],
                    kind.color(),
                )
            })
            .collect();

        // キャンバス（右側にカラーバー分の余白を確保。バー＋数値目盛＋縦書きタイトル分。
        // observed_contour.rs の COLORBAR_RESERVE と同じ幅を確保する）
        let avail = ui.available_size();
        let canvas_size = egui::vec2((avail.x - 96.0).max(120.0), avail.y.max(160.0));
        ui.allocate_ui(canvas_size, |ui| {
            ui.set_min_size(canvas_size);
            let (painter, rect, project, _click_pos, _hover_pos) = setup_3d_canvas(ui, camera);
            draw_3d_grid(&painter, &project);
            // 軸線は細分化してサーフェスと一緒に深度ソートし、面との前後関係を反映する
            draw_surface_mesh(
                &painter,
                &project,
                values,
                (v_min, v_max),
                (c_min, c_max),
                &cmap,
                bands.as_ref().map(|(lower, upper)| (lower, upper)),
                &observed_clip,
                &axis_segments_3d(24),
            );
            // 軸ラベルは読めるよう常に最前面に描く。
            // X = param1, Y(縦) = 目的関数値, Z = param2
            draw_3d_axis_labels(
                &painter,
                &project,
                [&result.param1_name, &value_label, &result.param2_name],
                [(x_min, x_max), (v_min, v_max), (y_min, y_max)],
            );

            // カラーバーはキャンバス右脇に重ねて描画する（色 = Mean の値域）。
            // ヒートマップ・contour と同じ共有描画（observed_contour.rs）を使う。
            let bar_rect = egui::Rect::from_min_size(
                egui::pos2(rect.right() + 6.0, rect.top()),
                egui::vec2(14.0, rect.height()),
            );
            draw_colorbar_simple(ui, bar_rect, c_min, c_max, cmap.clone(), Some(&value_label));
        });
    }
}

/// Mean グリッドと分散グリッドから 95% CI の下限・上限グリッドを作る。
/// 分散が数値誤差で負の場合は 0 として扱う（NaN を生まない）。
/// 不揃いな行は短い方に合わせる。
pub(crate) fn band_grids(z_values: &[Vec<f64>], variances: &[Vec<f64>]) -> BandGrids {
    let mut lower = Vec::with_capacity(z_values.len());
    let mut upper = Vec::with_capacity(z_values.len());
    for (z_row, var_row) in z_values.iter().zip(variances.iter()) {
        let mut l_row = Vec::with_capacity(z_row.len());
        let mut u_row = Vec::with_capacity(z_row.len());
        for (&z, &var) in z_row.iter().zip(var_row.iter()) {
            let sigma = var.max(0.0).sqrt();
            l_row.push(z - 1.96 * sigma);
            u_row.push(z + 1.96 * sigma);
        }
        lower.push(l_row);
        upper.push(u_row);
    }
    (lower, upper)
}

/// view から観測データ ([param1, param2, objective], 分類) を抽出する（テスト可能な純粋関数）。
///
/// フィルタ規則は 1D PDP の `extract_observed` と同じ:
/// `selected_indices` が空なら全試行、そうでなければ selected / pinned のみ。
/// 非有限値を含む行はスキップする。
/// 分類は他の散布図と同じ規則（pareto_rank == 0 → Pareto、is_feasible <= 0.5 → Infeasible）。
pub fn extract_observed_3d(
    view: &StudyView,
    param1: &str,
    param2: &str,
    objective: &str,
    selected_indices: &[u32],
    pinned: &[u32],
) -> Vec<([f64; 3], ObservedKind)> {
    let (Some(p1_col), Some(p2_col), Some(obj_col)) = (
        view.numeric_column(param1),
        view.numeric_column(param2),
        view.numeric_column(objective),
    ) else {
        return vec![];
    };
    let feas = view.feasibility();

    let use_filter = !selected_indices.is_empty();
    let selected_set: std::collections::HashSet<u32> = selected_indices.iter().copied().collect();
    let pinned_set: std::collections::HashSet<u32> = pinned.iter().copied().collect();

    (0..view.row_count())
        .filter_map(|i| {
            let trial_id = view.trial_ids.get(i).copied().unwrap_or(i as u32);
            if use_filter && !selected_set.contains(&trial_id) && !pinned_set.contains(&trial_id) {
                return None;
            }
            let p1 = p1_col.get(i).copied()?;
            let p2 = p2_col.get(i).copied()?;
            let ov = obj_col.get(i).copied()?;
            if !p1.is_finite() || !p2.is_finite() || !ov.is_finite() {
                return None;
            }
            let rank = view.pareto_rank.get(i).copied().unwrap_or(0);
            Some(([p1, p2, ov], classify_observed(feas.is_feasible(i), rank)))
        })
        .collect()
}

/// グリッド値をクリップ空間 [-1,1]^3 の四角形メッシュに変換する。
/// 座標系: x = 行 index（param1）、y = 値（縦軸）、z = 列 index（param2）。
/// 戻り値は (角4点のクリップ座標, セル平均値)。不揃いな行はスキップする。
pub(crate) fn surface_quads(
    values: &[Vec<f64>],
    v_min: f64,
    v_max: f64,
) -> Vec<([[f32; 3]; 4], f64)> {
    let n_row = values.len();
    if n_row < 2 {
        return Vec::new();
    }
    let clip_at = |row: usize, col: usize, n_col: usize| -> [f32; 3] {
        let x = 2.0 * row as f32 / (n_row - 1) as f32 - 1.0;
        let z = 2.0 * col as f32 / (n_col - 1) as f32 - 1.0;
        let y = normalize_to_clip(values[row][col], v_min, v_max);
        [x, y, z]
    };

    let n_col = values[0].len();
    if n_col < 2 {
        return Vec::new();
    }
    let mut quads = Vec::with_capacity((n_row - 1) * (n_col - 1));
    for row in 0..n_row - 1 {
        for col in 0..n_col - 1 {
            if values[row].len() <= col + 1 || values[row + 1].len() <= col + 1 {
                continue;
            }
            let corners = [
                clip_at(row, col, n_col),
                clip_at(row, col + 1, n_col),
                clip_at(row + 1, col + 1, n_col),
                clip_at(row + 1, col, n_col),
            ];
            let mean = (values[row][col]
                + values[row][col + 1]
                + values[row + 1][col]
                + values[row + 1][col + 1])
                / 4.0;
            quads.push((corners, mean));
        }
    }
    quads
}

/// 三角形を生メッシュに追加する
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

/// 線分をクアッド（三角形 2 枚）として生メッシュに追加する
fn push_edge(
    mesh: &mut egui::Mesh,
    a: egui::Pos2,
    b: egui::Pos2,
    color: egui::Color32,
    half_width: f32,
) {
    let v = b - a;
    let len = v.length();
    if len < f32::EPSILON {
        return;
    }
    let n = egui::vec2(-v.y, v.x) * (half_width / len);
    push_tri(mesh, [a + n, b + n, b - n], color);
    push_tri(mesh, [a + n, b - n, a - n], color);
}

/// サーフェスメッシュ・不確実性バンド・観測点を描画する。
///
/// セルを投影して深度ソートし、奥から手前へ塗る（painter's algorithm）。
/// 投影後の四角形は視点によって非凸・極端に細い形状になり得るが、egui の
/// テッセレータ（`Shape::convex_polygon` / ストローク）は鋭角でマイター法線が
/// 発散し画面を横切るスパイクを生むため、頂点座標をそのまま使う生の
/// `egui::Mesh` を直接構築する（法線計算が無いのでどんな退化形状でも安全）。
/// メッシュ線も細いクアッドとして同じメッシュに入れ、描画順を保つ。
/// バンド（半透明・メッシュ線なし）・観測点・3D 線分（軸線）も同じ深度リストに
/// 混ぜることで、重なりのブレンドや面の裏に隠れる前後関係が正しくなる。
#[allow(clippy::too_many_arguments)]
fn draw_surface_mesh(
    painter: &egui::Painter,
    project: &impl Fn([f32; 3]) -> (egui::Pos2, f32),
    values: &[Vec<f64>],
    clip_range: (f64, f64),
    color_range: (f64, f64),
    cmap: &ColorMap,
    bands: Option<(&Grid, &Grid)>,
    points: &[([f32; 3], egui::Color32)],
    lines: &[([f32; 3], [f32; 3], egui::Color32)],
) {
    enum Prim {
        Cell {
            corners: [egui::Pos2; 4],
            color: egui::Color32,
            edges: bool,
        },
        Point(egui::Pos2, egui::Color32),
        Line(egui::Pos2, egui::Pos2, egui::Color32),
    }

    let (v_min, v_max) = clip_range;
    let (c_min, c_max) = color_range;
    let mut items: Vec<(f32, Prim)> = Vec::new();

    // グリッドのセルを投影して深度リストへ追加する。
    // `color` が Some なら固定色（バンド）、None ならカラーマップ（Mean 面）。
    let collect_cells =
        |items: &mut Vec<(f32, Prim)>, grid: &[Vec<f64>], flat_color: Option<egui::Color32>| {
            for (corners, mean) in surface_quads(grid, v_min, v_max) {
                let mut pts = [egui::Pos2::ZERO; 4];
                let mut depth = 0.0;
                let mut finite = true;
                for (i, c) in corners.iter().enumerate() {
                    let (p, d) = project(*c);
                    finite &= p.x.is_finite() && p.y.is_finite();
                    pts[i] = p;
                    depth += d;
                }
                // 非有限値（NaN グリッドなど）を含むセルは描画しない
                if !finite {
                    continue;
                }
                let color = flat_color
                    .unwrap_or_else(|| cmap.interpolate(normalize_value(mean, c_min, c_max)));
                items.push((
                    depth * 0.25,
                    Prim::Cell {
                        corners: pts,
                        color,
                        edges: flat_color.is_none(),
                    },
                ));
            }
        };

    collect_cells(&mut items, values, None);
    if let Some((lower, upper)) = bands {
        collect_cells(&mut items, lower, Some(COLOR_PDP_CI));
        collect_cells(&mut items, upper, Some(COLOR_PDP_CI));
    }

    for (p, color) in points {
        let (pos, depth) = project(*p);
        if pos.x.is_finite() && pos.y.is_finite() {
            items.push((depth, Prim::Point(pos, *color)));
        }
    }

    for (a, b, color) in lines {
        let (pos_a, depth_a) = project(*a);
        let (pos_b, depth_b) = project(*b);
        let finite = pos_a.x.is_finite()
            && pos_a.y.is_finite()
            && pos_b.x.is_finite()
            && pos_b.y.is_finite();
        if finite {
            items.push(((depth_a + depth_b) * 0.5, Prim::Line(pos_a, pos_b, *color)));
        }
    }

    items.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

    let mut mesh = egui::Mesh::default();
    for (_, prim) in &items {
        match prim {
            Prim::Cell {
                corners,
                color,
                edges,
            } => {
                let [p0, p1, p2, p3] = *corners;
                push_tri(&mut mesh, [p0, p1, p2], *color);
                push_tri(&mut mesh, [p0, p2, p3], *color);
                // メッシュ線（セル外周のみ。対角線は描かない）
                if *edges {
                    let edge_color = color.gamma_multiply(0.6);
                    for (a, b) in [(p0, p1), (p1, p2), (p2, p3), (p3, p0)] {
                        push_edge(&mut mesh, a, b, edge_color, 0.35);
                    }
                }
            }
            Prim::Point(pos, color) => {
                // 円 Shape を挟むため、ここまでのメッシュを先に確定する
                if !mesh.is_empty() {
                    painter.add(egui::Shape::mesh(std::mem::take(&mut mesh)));
                }
                painter.circle_filled(*pos, 3.0, *color);
            }
            Prim::Line(a, b, color) => {
                push_edge(&mut mesh, *a, *b, *color, 0.75);
            }
        }
    }
    if !mesh.is_empty() {
        painter.add(egui::Shape::mesh(mesh));
    }
}

/// 軸グリッド値（昇順 linspace）から値域 [min, max] を返す
fn axis_range_of(values: &[f64]) -> (f64, f64) {
    match range_math::value_range(values.iter().copied()) {
        Some((mn, mx)) if mn.is_finite() && mx.is_finite() => (mn, mx),
        _ => (-1.0, 1.0),
    }
}

/// 値を [0.0, 1.0] に正規化する
pub fn normalize_value(v: f64, v_min: f64, v_max: f64) -> f32 {
    range_math::normalize01(v, v_min, v_max)
}

/// 値グリッドの値域 [min, max] を返す。
/// `value_range_of` は退化範囲（min==max）を拡張しない点が heatmap 側の
/// `value_range` と異なるため、共有ヘルパーの degenerate 拡張は使わない。
pub fn value_range_of(values: &[Vec<f64>]) -> (f64, f64) {
    range_math::value_range(values.iter().flatten().copied()).unwrap_or((0.0, 1.0))
}

/// param1 と param2 が異なることを確認する（同一の場合 false）
pub fn check_params_different(p1: &str, p2: &str) -> bool {
    !p1.is_empty() && !p2.is_empty() && p1 != p2
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_value_midpoint() {
        let t = normalize_value(0.5, 0.0, 1.0);
        assert!((t - 0.5).abs() < 1e-5);
    }

    #[test]
    fn normalize_value_clamps_below_zero() {
        let t = normalize_value(-1.0, 0.0, 1.0);
        assert_eq!(t, 0.0);
    }

    #[test]
    fn normalize_value_clamps_above_one() {
        let t = normalize_value(2.0, 0.0, 1.0);
        assert_eq!(t, 1.0);
    }

    #[test]
    fn normalize_value_equal_range_returns_half() {
        let t = normalize_value(5.0, 5.0, 5.0);
        assert_eq!(t, 0.5);
    }

    #[test]
    fn value_range_of_correct() {
        let grid = vec![vec![1.0, 3.0], vec![2.0, 0.5]];
        let (v_min, v_max) = value_range_of(&grid);
        assert!((v_min - 0.5).abs() < 1e-9);
        assert!((v_max - 3.0).abs() < 1e-9);
    }

    #[test]
    fn value_range_of_empty_returns_default() {
        let grid: Vec<Vec<f64>> = vec![];
        let (v_min, v_max) = value_range_of(&grid);
        assert_eq!(v_min, 0.0);
        assert_eq!(v_max, 1.0);
    }

    #[test]
    fn check_params_different_true_for_different() {
        assert!(check_params_different("x", "y"));
    }

    #[test]
    fn check_params_different_false_for_same() {
        assert!(!check_params_different("x", "x"));
    }

    #[test]
    fn check_params_different_false_for_empty() {
        assert!(!check_params_different("", "y"));
        assert!(!check_params_different("x", ""));
    }

    #[test]
    fn surface_quads_count_matches_grid_cells() {
        // 3x4 グリッド → (3-1)*(4-1) = 6 セル
        let grid = vec![
            vec![0.0, 1.0, 2.0, 3.0],
            vec![1.0, 2.0, 3.0, 4.0],
            vec![2.0, 3.0, 4.0, 5.0],
        ];
        let quads = surface_quads(&grid, 0.0, 5.0);
        assert_eq!(quads.len(), 6);
    }

    #[test]
    fn surface_quads_corners_span_clip_space() {
        let grid = vec![vec![0.0, 1.0], vec![1.0, 2.0]];
        let quads = surface_quads(&grid, 0.0, 2.0);
        assert_eq!(quads.len(), 1);
        let (corners, mean) = &quads[0];
        // x（行）・z（列）は [-1, 1] の端に乗る
        assert!((corners[0][0] - (-1.0)).abs() < 1e-6);
        assert!((corners[0][2] - (-1.0)).abs() < 1e-6);
        assert!((corners[2][0] - 1.0).abs() < 1e-6);
        assert!((corners[2][2] - 1.0).abs() < 1e-6);
        // y は値の正規化: 0.0 → -1, 2.0 → +1
        assert!((corners[0][1] - (-1.0)).abs() < 1e-6);
        assert!((corners[2][1] - 1.0).abs() < 1e-6);
        assert!((mean - 1.0).abs() < 1e-9);
    }

    #[test]
    fn surface_quads_empty_for_single_row_or_col() {
        assert!(surface_quads(&[vec![1.0, 2.0]], 0.0, 1.0).is_empty());
        assert!(surface_quads(&[vec![1.0], vec![2.0]], 0.0, 1.0).is_empty());
        let empty: Vec<Vec<f64>> = vec![];
        assert!(surface_quads(&empty, 0.0, 1.0).is_empty());
    }

    #[test]
    fn surface_quads_skips_ragged_rows() {
        // 2 行目が短い → 欠けたセルはスキップされる
        let grid = vec![vec![0.0, 1.0, 2.0], vec![1.0, 2.0], vec![2.0, 3.0, 4.0]];
        let quads = surface_quads(&grid, 0.0, 4.0);
        // (row0,col0) と (row1,col0) のみ有効（col1 は row1 が欠落）
        assert_eq!(quads.len(), 2);
    }

    #[test]
    fn pdp2d_default_camera_is_tilted() {
        let s = PdpChart2DState::default();
        assert!(!s.camera.is_identity_rotation());
        assert!(s.show_uncertainty);
        assert!(!s.show_observed);
        assert!(!s.feasible_only);
    }

    #[test]
    fn band_grids_computes_95_ci() {
        // 分散 4 → σ = 2 → ±1.96×2 = ±3.92
        let z = vec![vec![10.0, 20.0]];
        let var = vec![vec![4.0, 0.0]];
        let (lower, upper) = band_grids(&z, &var);
        assert!((lower[0][0] - (10.0 - 3.92)).abs() < 1e-9);
        assert!((upper[0][0] - (10.0 + 3.92)).abs() < 1e-9);
        // 分散 0 → バンドは Mean に一致
        assert_eq!(lower[0][1], 20.0);
        assert_eq!(upper[0][1], 20.0);
    }

    #[test]
    fn band_grids_negative_variance_does_not_produce_nan() {
        // ガウス過程の事後分散は数値誤差で僅かに負になり得る
        let z = vec![vec![5.0]];
        let var = vec![vec![-1e-12]];
        let (lower, upper) = band_grids(&z, &var);
        assert!(lower[0][0].is_finite());
        assert!(upper[0][0].is_finite());
        assert_eq!(lower[0][0], 5.0);
        assert_eq!(upper[0][0], 5.0);
    }

    #[test]
    fn band_grids_truncates_to_shorter_rows() {
        let z = vec![vec![1.0, 2.0], vec![3.0, 4.0]];
        let var = vec![vec![0.0]]; // 行数・列数とも不足
        let (lower, upper) = band_grids(&z, &var);
        assert_eq!(lower.len(), 1);
        assert_eq!(lower[0].len(), 1);
        assert_eq!(upper[0].len(), 1);
    }

    #[test]
    fn push_tri_appends_three_vertices_and_indices() {
        let mut mesh = egui::Mesh::default();
        push_tri(
            &mut mesh,
            [
                egui::pos2(0.0, 0.0),
                egui::pos2(1.0, 0.0),
                egui::pos2(0.0, 1.0),
            ],
            egui::Color32::RED,
        );
        assert_eq!(mesh.vertices.len(), 3);
        assert_eq!(mesh.indices, vec![0, 1, 2]);
    }

    #[test]
    fn push_edge_zero_length_adds_nothing() {
        let mut mesh = egui::Mesh::default();
        let p = egui::pos2(5.0, 5.0);
        push_edge(&mut mesh, p, p, egui::Color32::RED, 0.35);
        assert!(mesh.is_empty());
    }

    #[test]
    fn push_edge_builds_finite_quad() {
        // どんな線分でも頂点座標は有界（マイター発散のような無限大は生じない）
        let mut mesh = egui::Mesh::default();
        push_edge(
            &mut mesh,
            egui::pos2(0.0, 0.0),
            egui::pos2(100.0, 0.001),
            egui::Color32::RED,
            0.35,
        );
        assert_eq!(mesh.vertices.len(), 6);
        for v in &mesh.vertices {
            assert!(v.pos.x.is_finite() && v.pos.y.is_finite());
            assert!(v.pos.x.abs() <= 101.0 && v.pos.y.abs() <= 1.0);
        }
    }

    // ── extract_observed_3d ──────────────────────────────────────

    fn make_view_2p_ranked(p1: &[f64], p2: &[f64], obj: &[f64], ranks: Vec<u32>) -> StudyView {
        use std::collections::HashMap;
        use std::sync::Arc;
        use tunny_core::dataframe::{DataFrame, TrialRow as CoreRow};
        let n = obj.len();
        let param_names = vec!["p1".to_string(), "p2".to_string()];
        let obj_names = vec!["obj0".to_string()];
        let core_rows: Vec<CoreRow> = (0..n)
            .map(|i| CoreRow {
                trial_id: i as u32,
                trial_number: i as u32,
                param_display: [("p1".to_string(), p1[i]), ("p2".to_string(), p2[i])].into(),
                param_category_label: HashMap::new(),
                objective_values: vec![obj[i]],
                user_attrs_numeric: HashMap::new(),
                user_attrs_string: HashMap::new(),
                constraint_values: vec![],
            })
            .collect();
        let df = DataFrame::from_trials(&core_rows, &param_names, &obj_names, &[], &[], 0);
        StudyView::new(Arc::new(df), ranks)
    }

    fn make_view_2p(p1: &[f64], p2: &[f64], obj: &[f64]) -> StudyView {
        let n = obj.len();
        make_view_2p_ranked(p1, p2, obj, vec![0; n])
    }

    #[test]
    fn extract_observed_3d_returns_all_rows_without_selection() {
        let view = make_view_2p(&[1.0, 2.0], &[10.0, 20.0], &[0.5, 1.5]);
        let pts = extract_observed_3d(&view, "p1", "p2", "obj0", &[], &[]);
        assert_eq!(pts.len(), 2);
        assert_eq!(pts[0].0, [1.0, 10.0, 0.5]);
        assert_eq!(pts[1].0, [2.0, 20.0, 1.5]);
    }

    #[test]
    fn extract_observed_3d_filters_by_selection_and_pinned() {
        let view = make_view_2p(&[1.0, 2.0, 3.0], &[10.0, 20.0, 30.0], &[0.1, 0.2, 0.3]);
        let pts = extract_observed_3d(&view, "p1", "p2", "obj0", &[0], &[2]);
        let p1s: Vec<f64> = pts.iter().map(|(p, _)| p[0]).collect();
        assert!(p1s.contains(&1.0), "selected row must be visible");
        assert!(p1s.contains(&3.0), "pinned row must remain visible");
        assert!(
            !p1s.contains(&2.0),
            "unselected unpinned row must be hidden"
        );
    }

    #[test]
    fn extract_observed_3d_missing_column_returns_empty() {
        let view = make_view_2p(&[1.0], &[10.0], &[0.5]);
        assert!(extract_observed_3d(&view, "nope", "p2", "obj0", &[], &[]).is_empty());
        assert!(extract_observed_3d(&view, "p1", "p2", "nope", &[], &[]).is_empty());
    }

    #[test]
    fn extract_observed_3d_skips_non_finite_rows() {
        let view = make_view_2p(&[1.0, f64::NAN], &[10.0, 20.0], &[0.5, 1.5]);
        let pts = extract_observed_3d(&view, "p1", "p2", "obj0", &[], &[]);
        assert_eq!(pts.len(), 1);
        assert_eq!(pts[0].0, [1.0, 10.0, 0.5]);
    }

    #[test]
    fn extract_observed_3d_classifies_by_pareto_rank() {
        // rank 0 → Pareto（赤）、rank > 0 → NonPareto（青）
        let view = make_view_2p_ranked(&[1.0, 2.0], &[10.0, 20.0], &[0.5, 1.5], vec![0, 1]);
        let pts = extract_observed_3d(&view, "p1", "p2", "obj0", &[], &[]);
        assert_eq!(pts[0].1, ObservedKind::Pareto);
        assert_eq!(pts[1].1, ObservedKind::NonPareto);
    }
}
