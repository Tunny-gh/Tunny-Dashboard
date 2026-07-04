//! SOM（自己組織化マップ）ウィジェット。
//!
//! 標準化した特徴空間でバッチ SOM を学習し（`tunny_core::clustering::train_som`）、
//! U-matrix・成分プレーン・ヒットカウントを切り替えて表示する。学習はミリ秒〜
//! 数十ミリ秒オーダーのため SYNC ウィジェット（poll_chart を介さずレンダーパスで
//! 直接計算しキャッシュする）。理論的背景は theory/{en,ja}/clustering/som.md。
//!
//! 配線メモ（このファイルはまだ mod.rs に登録されていない。ChartId::SomMap /
//! label "SOM Map" / icon som_map.svg として配線予定）。

use crate::state::types::StudyView;
use crate::theme::colormap::ColorMap;
use crate::ui::widgets::common::heatmap::draw_colorbar_simple;
use crate::ui::widgets::common::range_math::{expand_degenerate, normalize01, value_range};
use tunny_core::clustering::{train_som, SomResult, SomSpec};

/// 学習に使う特徴空間。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum SomSpace {
    #[default]
    Params,
    ParamsAndObjectives,
}

impl SomSpace {
    fn label(self) -> &'static str {
        match self {
            SomSpace::Params => "Parameters",
            SomSpace::ParamsAndObjectives => "Parameters + Objectives",
        }
    }

    fn disc(self) -> u8 {
        match self {
            SomSpace::Params => 0,
            SomSpace::ParamsAndObjectives => 1,
        }
    }
}

/// 表示モード。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum SomViewMode {
    #[default]
    UMatrix,
    ComponentPlane,
    Hits,
}

impl SomViewMode {
    fn label(self) -> &'static str {
        match self {
            SomViewMode::UMatrix => "U-Matrix",
            SomViewMode::ComponentPlane => "Component Plane",
            SomViewMode::Hits => "Hit Counts",
        }
    }
}

/// SOM 学習に使う行数の上限（超える場合は等間隔ストライドでサブサンプル）。
const MAX_SOM_ROWS: usize = 2000;

/// (study_name, row_count, grid_size, n_epochs, space disc)
type SomCacheKey = (String, usize, usize, usize, u8);

/// SOM ウィジェットの UI 状態。
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct SomMapChart {
    /// グリッドの一辺のノード数（正方グリッド、両軸共通）。
    pub grid_size: usize,
    pub n_epochs: usize,
    pub view_mode: SomViewMode,
    /// ComponentPlane モードで表示する特徴名。
    pub selected_feature: String,
    pub space: SomSpace,
    #[serde(skip)]
    cache: Option<(SomCacheKey, SomResult)>,
}

impl Default for SomMapChart {
    fn default() -> Self {
        Self {
            grid_size: 8,
            n_epochs: 20,
            view_mode: SomViewMode::default(),
            selected_feature: String::new(),
            space: SomSpace::default(),
            cache: None,
        }
    }
}

/// `space` に応じた学習対象の特徴名一覧（目的関数を含めるかどうか）。
fn feature_names(param_names: &[String], obj_names: &[String], space: SomSpace) -> Vec<String> {
    match space {
        SomSpace::Params => param_names.to_vec(),
        SomSpace::ParamsAndObjectives => param_names
            .iter()
            .chain(obj_names.iter())
            .cloned()
            .collect(),
    }
}

/// 行数 `n` を `cap` 以下へ等間隔ストライドで間引くインデックス一覧（昇順・重複なし）。
/// `n <= cap` または `cap == 0` の場合は `0..n` をそのまま返す。
fn subsample_indices(n: usize, cap: usize) -> Vec<usize> {
    if cap == 0 || n <= cap {
        return (0..n).collect();
    }
    let step = n as f64 / cap as f64;
    (0..cap)
        .map(|i| ((i as f64 * step) as usize).min(n - 1))
        .collect()
}

/// view から学習行列を組み立てる。指定した全特徴が有限な行のみ採用し
/// （NaN 混入行はスキップ）、`MAX_SOM_ROWS` を超える場合は等間隔サブサンプルする。
fn build_matrix(view: &StudyView, features: &[String]) -> Vec<Vec<f64>> {
    let Some(cols): Option<Vec<&[f64]>> = features.iter().map(|f| view.numeric_column(f)).collect()
    else {
        return Vec::new();
    };
    let full_rows: Vec<Vec<f64>> = (0..view.row_count())
        .filter_map(|r| {
            cols.iter()
                .map(|c| c.get(r).copied().filter(|v| v.is_finite()))
                .collect::<Option<Vec<f64>>>()
        })
        .collect();
    let idx = subsample_indices(full_rows.len(), MAX_SOM_ROWS);
    idx.into_iter().map(|i| full_rows[i].clone()).collect()
}

impl SomMapChart {
    fn cache_key(&self, study_name: &str, row_count: usize) -> SomCacheKey {
        (
            study_name.to_string(),
            row_count,
            self.grid_size,
            self.n_epochs,
            self.space.disc(),
        )
    }

    /// 現在の表示モードに対応するノード値グリッド（行優先 `grid_h * grid_w`）と
    /// 軸ラベルを返す。CSV エクスポート（配線フェーズで csv_export.rs から呼ぶ想定）用。
    pub fn current_grid(
        &self,
        param_names: &[String],
        obj_names: &[String],
    ) -> Option<(usize, usize, Vec<f64>, String)> {
        let (_, result) = self.cache.as_ref()?;
        let features = feature_names(param_names, obj_names, self.space);
        let (values, label) = match self.view_mode {
            SomViewMode::UMatrix => (result.u_matrix.clone(), "u_matrix".to_string()),
            SomViewMode::Hits => (
                result.hits.iter().map(|&h| h as f64).collect(),
                "hits".to_string(),
            ),
            SomViewMode::ComponentPlane => {
                let idx = features.iter().position(|f| f == &self.selected_feature)?;
                (result.component_plane(idx), self.selected_feature.clone())
            }
        };
        Some((result.grid_w, result.grid_h, values, label))
    }

    #[allow(clippy::too_many_arguments)]
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        view: &StudyView,
        param_names: &[String],
        obj_names: &[String],
        study_name: &str,
        cmap: &ColorMap,
    ) {
        ui.horizontal(|ui| {
            ui.label("Grid size:");
            ui.add(egui::Slider::new(&mut self.grid_size, 4..=16));
            ui.label("Epochs:");
            egui::ComboBox::from_id_salt("som_epochs")
                .selected_text(self.n_epochs.to_string())
                .show_ui(ui, |ui| {
                    for n in [10usize, 20, 50] {
                        ui.selectable_value(&mut self.n_epochs, n, n.to_string());
                    }
                });
            egui::ComboBox::from_id_salt("som_space")
                .selected_text(self.space.label())
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.space,
                        SomSpace::Params,
                        SomSpace::Params.label(),
                    );
                    ui.selectable_value(
                        &mut self.space,
                        SomSpace::ParamsAndObjectives,
                        SomSpace::ParamsAndObjectives.label(),
                    );
                });
        });

        let features = feature_names(param_names, obj_names, self.space);

        ui.horizontal(|ui| {
            for mode in [
                SomViewMode::UMatrix,
                SomViewMode::ComponentPlane,
                SomViewMode::Hits,
            ] {
                if ui
                    .selectable_label(self.view_mode == mode, mode.label())
                    .clicked()
                {
                    self.view_mode = mode;
                }
            }
            if self.view_mode == SomViewMode::ComponentPlane {
                if self.selected_feature.is_empty() || !features.contains(&self.selected_feature) {
                    if let Some(f) = features.first() {
                        self.selected_feature = f.clone();
                    }
                }
                egui::ComboBox::from_id_salt("som_feature")
                    .selected_text(self.selected_feature.as_str())
                    .show_ui(ui, |ui| {
                        for f in &features {
                            ui.selectable_value(&mut self.selected_feature, f.clone(), f);
                        }
                    });
            }
        });

        if features.is_empty() {
            ui.colored_label(
                crate::theme::chart_colors::COLOR_EMPTY_STATE,
                "No numeric columns available.",
            );
            return;
        }

        let key = self.cache_key(study_name, view.row_count());
        if self.cache.as_ref().map(|(k, _)| k) != Some(&key) {
            let matrix = build_matrix(view, &features);
            let spec = SomSpec {
                grid_w: self.grid_size,
                grid_h: self.grid_size,
                n_epochs: self.n_epochs,
            };
            self.cache = train_som(&matrix, &spec).map(|r| (key, r));
        }

        let Some((_, result)) = &self.cache else {
            ui.colored_label(
                crate::theme::chart_colors::COLOR_EMPTY_STATE,
                "Not enough data to train a SOM (need >= 3 rows and a 2x2+ grid).",
            );
            return;
        };

        let (values, value_label): (Vec<f64>, String) = match self.view_mode {
            SomViewMode::UMatrix => (result.u_matrix.clone(), "U-matrix distance".to_string()),
            SomViewMode::Hits => (
                result.hits.iter().map(|&h| h as f64).collect(),
                "Hits".to_string(),
            ),
            SomViewMode::ComponentPlane => {
                let idx = features
                    .iter()
                    .position(|f| f == &self.selected_feature)
                    .unwrap_or(0);
                (result.component_plane(idx), self.selected_feature.clone())
            }
        };

        render_grid(
            ui,
            result.grid_w,
            result.grid_h,
            &values,
            cmap,
            &value_label,
        );

        if self.view_mode == SomViewMode::UMatrix {
            ui.label(egui::RichText::new("U-matrix ridges = cluster boundaries").weak());
        }
    }
}

/// ノードグリッドをセル塗り + カラーバーで描画する（heatmap.rs の共有描画を利用）。
fn render_grid(
    ui: &mut egui::Ui,
    grid_w: usize,
    grid_h: usize,
    values: &[f64],
    cmap: &ColorMap,
    label: &str,
) {
    let (v_min, v_max) = value_range(values.iter().copied())
        .map(|(mn, mx)| expand_degenerate(mn, mx))
        .unwrap_or((0.0, 1.0));

    let avail = ui.available_size();
    let side = (avail.x - 96.0)
        .max(120.0)
        .min(avail.y.max(160.0))
        .min(420.0);
    let canvas_size = egui::vec2(side + 96.0, side.max(160.0));
    ui.allocate_ui(canvas_size, |ui| {
        ui.set_min_size(canvas_size);
        let (rect, _resp) = ui.allocate_exact_size(egui::vec2(side, side), egui::Sense::hover());
        let painter = ui.painter_at(rect);
        let cell_w = rect.width() / grid_w as f32;
        let cell_h = rect.height() / grid_h as f32;
        for gy in 0..grid_h {
            for gx in 0..grid_w {
                let node = gy * grid_w + gx;
                let Some(&v) = values.get(node) else {
                    continue;
                };
                let t = normalize01(v, v_min, v_max);
                let color = cmap.interpolate(t);
                let cell_rect = egui::Rect::from_min_size(
                    egui::pos2(
                        rect.left() + gx as f32 * cell_w,
                        rect.top() + gy as f32 * cell_h,
                    ),
                    egui::vec2(cell_w + 1.0, cell_h + 1.0),
                );
                painter.rect_filled(cell_rect, 0.0, color);
            }
        }
        painter.rect_stroke(
            rect,
            0.0,
            egui::Stroke::new(1.0, egui::Color32::from_gray(120)),
            egui::StrokeKind::Inside,
        );

        let bar_rect = egui::Rect::from_min_size(
            egui::pos2(rect.right() + 12.0, rect.top()),
            egui::vec2(14.0, rect.height()),
        );
        draw_colorbar_simple(ui, bar_rect, v_min, v_max, cmap.clone(), Some(label));
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn som_map_chart_default_values() {
        let s = SomMapChart::default();
        assert_eq!(s.grid_size, 8);
        assert_eq!(s.n_epochs, 20);
        assert_eq!(s.view_mode, SomViewMode::UMatrix);
        assert_eq!(s.space, SomSpace::Params);
        assert!(s.selected_feature.is_empty());
        assert!(s.cache.is_none());
    }

    #[test]
    fn som_space_disc_is_distinct() {
        assert_ne!(
            SomSpace::Params.disc(),
            SomSpace::ParamsAndObjectives.disc()
        );
    }

    #[test]
    fn feature_names_params_only() {
        let params = vec!["x".to_string(), "y".to_string()];
        let objs = vec!["obj".to_string()];
        let names = feature_names(&params, &objs, SomSpace::Params);
        assert_eq!(names, params);
    }

    #[test]
    fn feature_names_params_and_objectives() {
        let params = vec!["x".to_string()];
        let objs = vec!["obj".to_string()];
        let names = feature_names(&params, &objs, SomSpace::ParamsAndObjectives);
        assert_eq!(names, vec!["x".to_string(), "obj".to_string()]);
    }

    #[test]
    fn subsample_indices_below_cap_returns_all() {
        assert_eq!(subsample_indices(5, 2000), (0..5).collect::<Vec<_>>());
    }

    #[test]
    fn subsample_indices_above_cap_returns_cap_count_ascending() {
        let idx = subsample_indices(10_000, 2000);
        assert_eq!(idx.len(), 2000);
        assert!(idx.windows(2).all(|w| w[0] <= w[1]));
        assert!(*idx.last().unwrap() < 10_000);
    }

    #[test]
    fn subsample_indices_zero_cap_returns_all() {
        assert_eq!(subsample_indices(3, 0), vec![0, 1, 2]);
    }

    #[test]
    fn build_matrix_skips_rows_with_non_finite_values() {
        use std::collections::HashMap;
        use std::sync::Arc;
        use tunny_core::dataframe::{DataFrame, TrialRow as CoreRow};

        let core_rows: Vec<CoreRow> = vec![
            CoreRow {
                trial_id: 0,
                trial_number: 0,
                param_display: [("x".to_string(), 1.0)].into(),
                param_category_label: HashMap::new(),
                objective_values: vec![],
                user_attrs_numeric: HashMap::new(),
                user_attrs_string: HashMap::new(),
                constraint_values: vec![],
            },
            CoreRow {
                trial_id: 1,
                trial_number: 1,
                param_display: [("x".to_string(), f64::NAN)].into(),
                param_category_label: HashMap::new(),
                objective_values: vec![],
                user_attrs_numeric: HashMap::new(),
                user_attrs_string: HashMap::new(),
                constraint_values: vec![],
            },
        ];
        let param_names = vec!["x".to_string()];
        let df = DataFrame::from_trials(&core_rows, &param_names, &[], &[], &[], 0);
        let view = StudyView::new(Arc::new(df), vec![0, 0]);
        let matrix = build_matrix(&view, &param_names);
        assert_eq!(matrix.len(), 1);
        assert_eq!(matrix[0], vec![1.0]);
    }
}
